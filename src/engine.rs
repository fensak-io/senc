// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::borrow::{Borrow, Cow};
use std::collections;
use std::fs;
use std::io::Write;
use std::path;
use std::rc::Rc;
use std::vec;

use anyhow::{anyhow, Result};
use deno_core::*;

use crate::files;
use crate::module_loader;
use crate::ops;

// Load and embed the runtime snapshot built from the build script.
static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/SENC_SNAPSHOT.bin"));

// The runtime context, containing various metadata that is used by the builtin operations.
#[derive(Clone)]
pub struct Context {
    pub node_modules_dir: Option<path::PathBuf>,
    pub projectroot: path::PathBuf,
    pub out_dir: path::PathBuf,
}

// A request to run a single JS/TS file through.
pub struct RunRequest {
    pub in_file: String,
    pub out_file_stem: String,
}

// The data to be written to disk, including the file extension to use.
pub struct OutData {
    // The output file path. If set, this will override the default output file path that is based
    // on the input file and project root.
    //
    // When set, this must be a relative path. The relative path will be relative to the original
    // output directory. The path can traverse outside the default directory, but it can not
    // traverse outside of the project root.
    //
    // Exactly one of out_path or out_ext may be set.
    out_path: Option<String>,

    // The extension of the output file, including the preceding `.`
    // Exactly one of out_path or out_ext may be set.
    out_ext: Option<String>,

    // The full, raw string contents of the output file.
    data: String,
}

// The output types supported
enum OutputType {
    JSON,
    YAML,
}

impl std::fmt::Display for RunRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "request to run {} to generate {}",
            self.in_file, self.out_file_stem
        )
    }
}

// Initialize the v8 platform. This should be called in the main thread before any subthreads are
// launched.
pub fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    JsRuntime::init_platform(Some(platform));
}

pub async fn run_js_and_write(ctx: &Context, req: &RunRequest) -> Result<()> {
    let out_data_vec = run_js(ctx, req).await?;
    for d in out_data_vec {
        // TODO
        // collect the errors and return one big error instead of failing fast
        write_data(ctx.out_dir.as_path(), &req.out_file_stem, &d)?;
    }
    return Ok(());
}

// Run the javascript or typescript file available at the given file path through the Deno runtime.
async fn run_js(ctx: &Context, req: &RunRequest) -> Result<vec::Vec<OutData>> {
    let mut js_runtime = new_runtime(ctx, req)?;
    let mod_id = load_main_module(&mut js_runtime, &req.in_file).await?;
    let main_fn = load_main_fn(&mut js_runtime, mod_id)?;
    let result = js_runtime.call_and_await(&main_fn).await?;
    return load_result(&mut js_runtime, result);
}

fn new_runtime(ctx: &Context, req: &RunRequest) -> Result<JsRuntime> {
    let opext = Extension {
        name: "opbuiltins",
        ops: Cow::Borrowed(&[
            ops::op_log_trace::DECL,
            ops::op_log_debug::DECL,
            ops::op_log_info::DECL,
            ops::op_log_warn::DECL,
            ops::op_log_error::DECL,
            ops::op_path_relpath::DECL,
        ]),
        middleware_fn: Some(Box::new(|op| match op.name {
            "op_print" => op.disable(),
            _ => op,
        })),
        ..Default::default()
    };
    let tmplext = load_templated_builtins(ctx, req)?;
    Ok(JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(module_loader::TsModuleLoader::new(
            ctx.node_modules_dir.clone(),
        ))),
        extensions: vec![opext, tmplext],
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        ..Default::default()
    }))
}

async fn load_main_module(js_runtime: &mut JsRuntime, file_path: &str) -> Result<usize> {
    let main_module = resolve_path(file_path, std::env::current_dir()?.as_path())?;
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(false).await?;
    result.await??;
    return Ok(mod_id);
}

fn load_main_fn(js_runtime: &mut JsRuntime, mod_id: usize) -> Result<v8::Global<v8::Function>> {
    let ns = js_runtime.get_module_namespace(mod_id)?;
    let mut scope = js_runtime.handle_scope();
    let main_fn_key = v8::String::new(&mut scope, "main").unwrap();
    let main_fn_local: v8::Local<v8::Function> = ns
        .open(&mut scope)
        .get(&mut scope, main_fn_key.into())
        .unwrap()
        .try_into()?;
    let main_fn = v8::Global::new(&mut scope, main_fn_local);
    return Ok(main_fn);
}

fn load_result(
    js_runtime: &mut JsRuntime,
    result: v8::Global<v8::Value>,
) -> Result<vec::Vec<OutData>> {
    let mut out: vec::Vec<OutData> = vec::Vec::new();

    let mut scope = &mut js_runtime.handle_scope();
    let result_local = v8::Local::new(&mut scope, result);

    // Determine if the raw JS object from the runtime is an out data list object, in which case
    // each element needs to be cycled and converted.
    if result_is_senc_out_data_array(&mut scope, result_local)? {
        let result_arr: v8::Local<v8::Array> = result_local.try_into()?;
        let result_arr_raw: &v8::Array = result_arr.borrow();
        let sz = result_arr_raw.length();
        for i in 0..sz {
            let item = result_arr_raw.get_index(&mut scope, i).unwrap();
            let single_out = load_one_result(&mut scope, item)?;
            out.push(single_out);
        }
    } else {
        let single_out = load_one_result(&mut scope, result_local)?;
        out.push(single_out);
    }

    return Ok(out);
}

fn load_one_result(
    scope: &mut v8::HandleScope,
    orig_result_local: v8::Local<v8::Value>,
) -> Result<OutData> {
    let mut result_local = orig_result_local.clone();
    let mut out_path: Option<String> = None;
    let mut out_ext = Some(String::from(".json"));
    let mut out_type = OutputType::JSON;

    // Determine if the raw JS object from the runtime is an out data object.
    if result_is_senc_out_data(scope, result_local)? {
        let result_obj: v8::Local<v8::Object> = result_local.try_into()?;
        let out_type_key: v8::Local<v8::Value> = v8::String::new(scope, "out_type").unwrap().into();
        let out_type_local: v8::Local<v8::String> =
            result_obj.get(scope, out_type_key).unwrap().try_into()?;
        let out_type_str: &str = &out_type_local.to_rust_string_lossy(scope);
        match out_type_str {
            "yaml" => {
                out_type = OutputType::YAML;
            }
            "" | "json" => {} // Use default
            s => return Err(anyhow!("out_type {s} in OutData object is not supported")),
        }

        let out_path_key: v8::Local<v8::Value> = v8::String::new(scope, "out_path").unwrap().into();
        let out_ext_key: v8::Local<v8::Value> = v8::String::new(scope, "out_ext").unwrap().into();
        let maybe_out_path: v8::Local<v8::Value> = result_obj.get(scope, out_path_key).unwrap();
        let maybe_out_ext: v8::Local<v8::Value> = result_obj.get(scope, out_ext_key).unwrap();

        if maybe_out_path.is_string() && maybe_out_ext.is_string() {
            return Err(anyhow!(
                "OutData object can only have at most one of out_path or out_ext set, but not both"
            ));
        } else if maybe_out_path.is_string() {
            let out_path_local: v8::Local<v8::String> = maybe_out_path.try_into()?;
            out_path = Some(out_path_local.to_rust_string_lossy(scope));
            out_ext = None;
        } else if maybe_out_ext.is_string() {
            let out_ext_local: v8::Local<v8::String> = maybe_out_ext.try_into()?;
            out_ext = Some(out_ext_local.to_rust_string_lossy(scope));
            out_path = None;
        }

        let out_data_key: v8::Local<v8::Value> = v8::String::new(scope, "data").unwrap().into();
        result_local = result_obj.get(scope, out_data_key).unwrap().try_into()?;
    }

    let data = match out_type {
        OutputType::JSON => {
            let deserialized_result = serde_v8::from_v8::<serde_json::Value>(scope, result_local)?;
            serde_json::to_string(&deserialized_result)?.to_string()
        }
        OutputType::YAML => {
            let deserialized_result = serde_v8::from_v8::<serde_yaml::Value>(scope, result_local)?;
            serde_yaml::to_string(&deserialized_result)?.to_string()
        }
    };
    return Ok(OutData {
        out_path,
        out_ext,
        data,
    });
}

fn result_is_senc_out_data(
    scope: &mut v8::HandleScope,
    result_local: v8::Local<v8::Value>,
) -> Result<bool> {
    if !result_local.is_object() {
        return Ok(false);
    }

    let result_obj: v8::Local<v8::Object> = result_local.try_into()?;
    let is_senc_out_data_key: v8::Local<v8::Value> = v8::String::new(scope, "__is_senc_out_data")
        .unwrap()
        .try_into()?;
    return Ok(result_obj.has(scope, is_senc_out_data_key).unwrap());
}

fn result_is_senc_out_data_array(
    scope: &mut v8::HandleScope,
    result_local: v8::Local<v8::Value>,
) -> Result<bool> {
    if !result_local.is_array() {
        return Ok(false);
    }

    let result_arr: v8::Local<v8::Array> = result_local.try_into()?;
    let is_senc_out_data_array_key: v8::Local<v8::Value> =
        v8::String::new(scope, "__is_senc_out_data_array")
            .unwrap()
            .try_into()?;
    return Ok(result_arr.has(scope, is_senc_out_data_array_key).unwrap());
}

fn write_data(out_dir: &path::Path, out_file_stem: &str, data: &OutData) -> Result<()> {
    let mut out_file_path_str = String::new();
    if let Some(out_path) = &data.out_path {
        let mut out_file_stem_dir = path::PathBuf::from(out_file_stem)
            .parent()
            .unwrap()
            .to_owned();
        out_file_stem_dir.push(&out_path);
        out_file_path_str.push_str(&out_file_stem_dir.to_string_lossy());
    } else {
        out_file_path_str.push_str(out_file_stem);
        out_file_path_str.push_str(&data.out_ext.clone().unwrap());
    }
    let out_file_path = path_clean::clean(path::PathBuf::from(out_file_path_str));
    files::assert_file_path_in_projectroot(&out_file_path, out_dir)?;

    let out_file_dir = out_file_path.parent().unwrap();
    fs::create_dir_all(out_file_dir)?;
    let mut f = fs::File::create(out_file_path)?;
    f.write_all(data.data.as_bytes())?;

    return Ok(());
}

fn load_templated_builtins(ctx: &Context, req: &RunRequest) -> Result<Extension> {
    let mut hbs = handlebars::Handlebars::new();
    let staticpath_tmpl = include_str!("templated_builtins/staticpath.js.hbs");
    hbs.register_template_string("t1", staticpath_tmpl)?;

    let mut hbdata = collections::BTreeMap::new();
    hbdata.insert(
        "projectroot".to_string(),
        ctx.projectroot.to_string_lossy().to_string(),
    );
    hbdata.insert("filename".to_string(), req.in_file.clone());
    hbdata.insert(
        "dirname".to_string(),
        path::PathBuf::from(req.in_file.clone())
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );
    let rendered = hbs.render("t1", &hbdata).unwrap();

    let specifier = "ext:builtins/staticpath.js";
    let code = ExtensionFileSourceCode::Computed(rendered.into());
    let files = vec![ExtensionFileSource { specifier, code }];
    let ext = Extension {
        name: "templatedbuiltins",
        esm_entry_point: Some(specifier),
        esm_files: Cow::Owned(files),
        ..Default::default()
    };
    Ok(ext)
}

// Test cases

#[cfg(test)]
mod tests {
    use super::*;

    static EXPECTED_SIMPLE_OUTPUT_JSON: &str =
        "{\"foo\":\"bar\",\"fizz\":42,\"obj\":{\"msg\":\"hello world\"}}";
    static EXPECTED_LODASH_OUTPUT_JSON: &str = "{\"foo\":\"bar\",\"cfg\":false}";
    static EXPECTED_RELPATH_OUTPUT_JSON: &str =
        "{\"state\":\"aws/us-east-1/vpc/terraform.tfstate\",\"mainf\":\"aws/us-east-1/vpc/main.js\"}";
    static EXPECTED_OUTFILE_OUTPUT_JSON: &str = "{\"this\":\"outfile.js\"}";

    #[tokio::test]
    async fn test_engine_runs_js() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_SIMPLE_OUTPUT_JSON)
            .expect("error unpacking simple expected output json");

        let p = get_fixture_path("simple.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);
        let od = &od_vec[0];

        assert_eq!(od.out_path, None);
        assert_eq!(od.out_ext, Some(String::from(".json")));
        let actual_output: serde_json::Value =
            serde_json::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    #[tokio::test]
    async fn test_engine_runs_ts() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_SIMPLE_OUTPUT_JSON)
            .expect("error unpacking simple expected output json");

        let p = get_fixture_path("simple.ts");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);
        let od = &od_vec[0];

        assert_eq!(od.out_path, None);
        assert_eq!(od.out_ext, Some(String::from(".json")));
        let actual_output: serde_json::Value =
            serde_json::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_node_modules() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_LODASH_OUTPUT_JSON)
            .expect("error unpacking lodash expected output json");

        let p = get_fixture_path("with_lodash.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);
        let od = &od_vec[0];

        assert_eq!(od.out_path, None);
        assert_eq!(od.out_ext, Some(String::from(".json")));
        let actual_output: serde_json::Value =
            serde_json::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_builtin_filefunctions() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_RELPATH_OUTPUT_JSON)
            .expect("error unpacking relpath output json");

        let p = get_fixture_path("aws/us-east-1/vpc/main.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);
        let od = &od_vec[0];

        assert_eq!(od.out_path, None);
        assert_eq!(od.out_ext, Some(String::from(".json")));
        let actual_output: serde_json::Value =
            serde_json::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_out_data_output() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_OUTFILE_OUTPUT_JSON)
            .expect("error unpacking outfile expected output json");

        let p = get_fixture_path("outfile.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);
        let od = &od_vec[0];

        assert_eq!(od.out_path, None);
        assert_eq!(od.out_ext, Some(String::from(".yml")));
        let actual_output: serde_json::Value =
            serde_yaml::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    fn get_context() -> Context {
        let node_modules_dir = Some(get_fixture_path("node_modules"));
        let projectroot = get_fixture_path("");
        let out_dir = get_fixture_path("");
        Context {
            node_modules_dir,
            projectroot,
            out_dir,
        }
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_out_data_list_output() {
        let expected_output: serde_json::Value = serde_json::from_str(EXPECTED_OUTFILE_OUTPUT_JSON)
            .expect("error unpacking outfile expected output json");

        let p = get_fixture_path("multi_outfile.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 2);

        let d1 = &od_vec[0];
        assert_eq!(d1.out_path, None);
        assert_eq!(d1.out_ext, Some(String::from(".yml")));
        let actual_output1: serde_json::Value =
            serde_yaml::from_str(&d1.data).expect("error unpacking js data");
        assert_eq!(actual_output1, expected_output);

        let d2 = &od_vec[1];
        assert_eq!(d2.out_path, None);
        assert_eq!(d2.out_ext, Some(String::from(".json")));
        let actual_output2: serde_json::Value =
            serde_json::from_str(&d2.data).expect("error unpacking js data");
        assert_eq!(actual_output2, expected_output);
    }

    fn get_fixture_path(relpath: &str) -> path::PathBuf {
        let mut p = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        if relpath != "" {
            p.push(relpath);
        }
        return p;
    }
}
