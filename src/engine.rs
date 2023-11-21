// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::borrow::{Borrow, Cow};
use std::collections;
use std::env;
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

impl std::fmt::Display for RunRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "request to run {} to generate {}",
            self.in_file, self.out_file_stem
        )
    }
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

    // Prefix to append to the output before writing to file.
    out_prefix: Option<String>,

    // The full, raw string contents of the output file.
    data: String,
}

// The output types supported
enum OutputType {
    JSON,
    YAML,
}

// Initialize the v8 platform. This should be called in the main thread before any subthreads are
// launched.
pub fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    JsRuntime::init_platform(Some(platform));
}

// Process the request to run the JavaScript or TypeScript file to render the output in to the
// configured output dir. This will run the script and then write the output to the computed
// destination in one step.
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

// Initialize a new JsRuntime object (which represents an Isolate) with all the extensions loaded.
fn new_runtime(ctx: &Context, req: &RunRequest) -> Result<JsRuntime> {
    let modloader =
        module_loader::TsModuleLoader::new(ctx.projectroot.clone(), ctx.node_modules_dir.clone());
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
    let opts = RuntimeOptions {
        module_loader: Some(Rc::new(modloader)),
        extensions: vec![opext, tmplext],
        // NOTE
        // This snapshot contains the builtins/*.js scripts and is constructed in the build.rs
        // script.
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        ..Default::default()
    };
    Ok(JsRuntime::new(opts))
}

// Load the main module. The main module is the main entrypoint that is being executed by senc.
async fn load_main_module(js_runtime: &mut JsRuntime, file_path: &str) -> Result<usize> {
    let main_module = resolve_path(file_path, std::env::current_dir()?.as_path())?;
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(false).await?;
    result.await??;
    return Ok(mod_id);
}

// Load the main function from the main module. This is the main function that is exported in the
// main module script.
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

// Load the result from the main function as a vector of OutData that can be outputed to disk. Each
// OutData represents a single file that should be outputed.
fn load_result(
    js_runtime: &mut JsRuntime,
    result: v8::Global<v8::Value>,
) -> Result<vec::Vec<OutData>> {
    let mut out: vec::Vec<OutData> = vec::Vec::new();

    let mut scope = &mut js_runtime.handle_scope();
    let result_local = v8::Local::new(&mut scope, result);

    // Determine if the raw JS object from the runtime is an out data list object, in which case
    // each element needs to be cycled and converted.
    if result_is_sencjs_out_data_array(&mut scope, result_local)? {
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

// Load a single result data item. This can handle either of the following:
// - An OutData object (in JS, not to be confused by the OutData struct defined in this file). This
//   allows customization of the output behavior on a file by file basis.
// - Anything else would be treated as raw object to be serialized to JSON.
fn load_one_result<'a>(
    scope: &mut v8::HandleScope<'a>,
    orig_result_local: v8::Local<'a, v8::Value>,
) -> Result<OutData> {
    let mut result_local = orig_result_local.clone();

    // Defaults
    let mut out_path: Option<String> = None;
    let mut out_ext = Some(String::from(".json"));
    let mut out_type = OutputType::JSON;
    let mut out_prefix: Option<String> = None;

    // Determine if the raw JS object from the runtime is an out data object, and if it is, process
    // it.
    if result_is_sencjs_out_data(scope, result_local)? {
        let (op, oe, ot, opre, rs) = load_one_sencjs_out_data_result(scope, result_local)?;
        out_path = op;
        out_ext = oe;
        out_type = ot;
        out_prefix = opre;
        result_local = rs;
    }

    let deserialized_result = serde_v8::from_v8::<serde_json::Value>(scope, result_local)?;
    let data = match out_type {
        // NOTE
        // Both serde_json and serde_yaml have consistent outputs, so we don't need to do anything
        // special
        OutputType::JSON => serde_json::to_string_pretty(&deserialized_result)?.to_string(),
        OutputType::YAML => serde_yaml::to_string(&deserialized_result)?.to_string(),
    };
    return Ok(OutData {
        out_path,
        out_ext,
        out_prefix,
        data,
    });
}

// Load a single result from the main function that is a JS OutData object (not to be confused with
// the OutData struct defined in this file).
fn load_one_sencjs_out_data_result<'a>(
    scope: &mut v8::HandleScope<'a>,
    result_local: v8::Local<'a, v8::Value>,
) -> Result<(
    // out_path
    Option<String>,
    // out_ext
    Option<String>,
    // out_type
    OutputType,
    // out_prefix
    Option<String>,
    // result_local
    v8::Local<'a, v8::Value>,
)> {
    let mut out_path: Option<String> = None;
    let mut out_ext: Option<String> = None;
    let mut out_type = OutputType::JSON;
    let mut out_prefix: Option<String> = None;

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
    let out_prefix_key: v8::Local<v8::Value> = v8::String::new(scope, "out_prefix").unwrap().into();
    let maybe_out_path: v8::Local<v8::Value> = result_obj.get(scope, out_path_key).unwrap();
    let maybe_out_ext: v8::Local<v8::Value> = result_obj.get(scope, out_ext_key).unwrap();
    let maybe_out_prefix: v8::Local<v8::Value> = result_obj.get(scope, out_prefix_key).unwrap();

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

    if maybe_out_prefix.is_string() {
        let out_prefix_local: v8::Local<v8::String> = maybe_out_prefix.try_into()?;
        out_prefix = Some(out_prefix_local.to_rust_string_lossy(scope));
    }

    let out_data_key: v8::Local<v8::Value> = v8::String::new(scope, "data").unwrap().into();
    Ok((
        out_path,
        out_ext,
        out_type,
        out_prefix,
        result_obj.get(scope, out_data_key).unwrap().try_into()?,
    ))
}

// Checks whether the result from the main function is a JS OutData object from senc.js. It is a JS
// OutData object if it is an Object and it has the `__is_senc_out_data` method.
fn result_is_sencjs_out_data(
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

// Checks whether the result from the main function is a JS OutDataArray object from senc.js. It is
// a JS OutDataArray object if it is an Array and it has the `__is_senc_out_data_array` method.
fn result_is_sencjs_out_data_array(
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

// Write the results of a single out data to disk. The output file path is determined by whether
// the OutData object sets an `out_path` or not:
//
// If `out_path` is set:
// - This will write directly to `out_path`. `out_path` is expected to be a path relative to the main
//   script dir in the out dir.
//
//   Since this is confusing, here is an example. If the project tree is:
//
//   out
//   example
//   ├── foo
//   │   └── bar
//   │       └── baz
//   │           └── main.sen.js
//   └── carol
//       └── main.sen.js
//
//   and the project root is `./example`, while the output dir is `./out`, then:
//   - The `out_path` for `./example/foo/bar/baz/main.sen.js` would be relative to
//     `./out/foo/bar/baz`
//   - The `out_path` for `./example/carol/main.sen.js` would be relative to `./out/carol`
//
//   Note that the `out_path` must be within the output dir. That is, while `../` is supported,
//   you can't go up high enough to escape the output directory.
//
// If out_ext is set:
// - This will write to the path joined by `out_file_stem` and `out_ext` in the output directory.
//   `out_file_stem` is the relative path from the project root to the main script, with the
//   `.sen.js` extension dropped.
//
//   In the above example for `out_path`, the output file path for
//   `./example/foo/bar/baz/main.sen.js` will be `./out/foo/bar/baz/main.json`
//
// This will create all necessary directories to write the output file.
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

    let mut tmp = data.data.clone();
    if let Some(pre) = &data.out_prefix {
        tmp.insert_str(0, &pre);
    };
    f.write_all(tmp.as_bytes())?;

    return Ok(());
}

// Load any runtime builtin functions that are templated. These are builtins that are dynamic to
// the context of the runtime (e.g., the path of the current main file).
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
    static EXPECTED_IMPORT_CONFIG_OUTPUT_JSON: &str = "{\"msg\":\"hello world\"}";

    #[tokio::test]
    async fn test_engine_runs_js() {
        check_single_json_output(EXPECTED_SIMPLE_OUTPUT_JSON, "simple.js").await;
    }

    #[tokio::test]
    async fn test_engine_runs_ts() {
        check_single_json_output(EXPECTED_SIMPLE_OUTPUT_JSON, "simple.ts").await;
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_config_json_import() {
        check_single_json_output(EXPECTED_IMPORT_CONFIG_OUTPUT_JSON, "import_json.js").await;
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_config_yaml_import() {
        check_single_json_output(EXPECTED_IMPORT_CONFIG_OUTPUT_JSON, "import_yaml.js").await;
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_node_modules() {
        check_single_json_output(EXPECTED_LODASH_OUTPUT_JSON, "with_lodash.js").await;
    }

    #[tokio::test]
    async fn test_engine_runs_code_with_builtin_filefunctions() {
        check_single_json_output(EXPECTED_RELPATH_OUTPUT_JSON, "aws/us-east-1/vpc/main.js").await;
    }

    #[tokio::test]
    async fn test_engine_fails_code_with_import_outside_projectroot() {
        let p = get_fixture_path("import_restricted_to_project_root.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let result = run_js(&get_context(), &req).await;
        assert!(result.is_err());
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
        assert_eq!(od.out_prefix, None);
        let actual_output: serde_json::Value =
            serde_yaml::from_str(&od.data).expect("error unpacking js data");
        assert_eq!(actual_output, expected_output);
    }

    #[tokio::test]
    async fn test_engine_json_consistent_output() {
        let p = get_fixture_path("simple.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let first_od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(first_od_vec.len(), 1);
        let first_od = &first_od_vec[0];

        for _i in 0..100 {
            let od_vec = run_js(&get_context(), &req)
                .await
                .expect("error running js");
            assert_eq!(od_vec.len(), 1);
            assert_eq!(od_vec[0].data, first_od.data);
        }
    }

    #[tokio::test]
    async fn test_engine_yaml_consistent_output() {
        let p = get_fixture_path("simple_yaml.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let first_od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(first_od_vec.len(), 1);
        let first_od = &first_od_vec[0];

        for _i in 0..100 {
            let od_vec = run_js(&get_context(), &req)
                .await
                .expect("error running js");
            assert_eq!(od_vec.len(), 1);
            assert_eq!(od_vec[0].data, first_od.data);
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
        assert_eq!(d1.out_prefix, None);
        let actual_output1: serde_json::Value =
            serde_yaml::from_str(&d1.data).expect("error unpacking yml data");
        assert_eq!(actual_output1, expected_output);

        let d2 = &od_vec[1];
        assert_eq!(d2.out_path, None);
        assert_eq!(d2.out_ext, Some(String::from(".json")));
        assert_eq!(d2.out_prefix, None);
        let actual_output2: serde_json::Value =
            serde_json::from_str(&d2.data).expect("error unpacking js data");
        assert_eq!(actual_output2, expected_output);
    }

    #[tokio::test]
    async fn test_engine_prepends_prefix_if_set() {
        let p = get_fixture_path("outfile_prefix.js");
        let req = RunRequest {
            in_file: String::from(p.as_path().to_string_lossy()),
            out_file_stem: String::from(""),
        };
        let mut od_vec = run_js(&get_context(), &req)
            .await
            .expect("error running js");
        assert_eq!(od_vec.len(), 1);

        let d = &mut od_vec[0];
        assert_eq!(d.out_prefix, Some(String::from("# this is a prefix\n")));

        // Write the out data to a temp file
        let temp_dir = env::temp_dir();
        let file_name = format!("{}.yml", uuid::Uuid::new_v4());
        let outf = temp_dir.join(&file_name);
        d.out_ext = None;
        d.out_path = Some(String::from(&file_name));
        write_data(&temp_dir, &outf.to_string_lossy(), d).expect("could not save output to disk");

        let do_steps = || -> Result<()> {
            // ... and confirm prefix is prepended to output.
            let code = fs::read_to_string(&outf).expect("did not write to output file");
            if !code.starts_with("# this is a prefix\n") {
                Err(anyhow!("output file does not have expected prefix"))
            } else {
                Ok(())
            }
        };
        let step_result = do_steps();
        // Remove the temp file before checking result.
        fs::remove_file(outf).expect("could not remove output file");
        let _ = step_result.expect("wrong output");
    }

    async fn check_single_json_output(output_json_str: &str, fixture_fname: &str) {
        let expected_output: serde_json::Value =
            serde_json::from_str(output_json_str).expect("error unpacking expected output json");

        let p = get_fixture_path(fixture_fname);
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
        assert_eq!(od.out_prefix, None);
        let actual_output: serde_json::Value =
            serde_json::from_str(&od.data).expect("error unpacking js data");
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

    fn get_fixture_path(relpath: &str) -> path::PathBuf {
        let mut p = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        if relpath != "" {
            p.push(relpath);
        }
        return p;
    }
}
