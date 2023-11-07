// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::rc::Rc;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::futures::FutureExt;
use deno_core::*;

// Load and embed the runtime snapshot built from the build script.
static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/SENC_SNAPSHOT.bin"));

// A request to run a single JS/TS file through.
pub struct RunRequest {
    pub in_file: String,
    pub out_file_stem: String,
}

// The data to be written to disk, including the file extension to use.
struct OutData {
    out_ext: String,
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

// Run the javascript or typescript file available at the given file path through the Deno runtime.
pub async fn run_js(req: &RunRequest) -> Result<()> {
    let mut js_runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(TsModuleLoader)),
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        ..Default::default()
    });

    let mod_id = load_main_module(&mut js_runtime, &req.in_file).await?;
    let main_fn = load_main_fn(&mut js_runtime, mod_id).unwrap();
    let result = js_runtime.call_and_await(&main_fn).await?;
    let out_data = load_result(&mut js_runtime, result).unwrap();
    return write_data(&req.out_file_stem, &out_data);
}

async fn load_main_module(
    js_runtime: &mut JsRuntime,
    file_path: &str,
) -> Result<usize> {
    let main_module =
        resolve_path(file_path, std::env::current_dir()?.as_path())?;
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(false).await?;
    result.await?.unwrap();
    return Ok(mod_id);
}

fn load_main_fn(
    js_runtime: &mut JsRuntime,
    mod_id: usize,
) -> Result<v8::Global<v8::Function>> {
    let ns = js_runtime.get_module_namespace(mod_id).unwrap();
    let mut scope = js_runtime.handle_scope();
    let main_fn_key = v8::String::new(&mut scope, "main").unwrap();
    let main_fn_local: v8::Local<v8::Function> = ns
        .open(&mut scope)
        .get(&mut scope, main_fn_key.into())
        .unwrap()
        .try_into()
        .unwrap();
    let main_fn = v8::Global::new(&mut scope, main_fn_local);
    return Ok(main_fn);
}

fn load_result(
    js_runtime: &mut JsRuntime,
    result: v8::Global<v8::Value>,
) -> Result<OutData> {
    let mut scope = &mut js_runtime.handle_scope();
    let mut result_local = v8::Local::new(&mut scope, result);

    let mut out_ext = ".json".to_string();
    let mut out_type = OutputType::JSON;

    // Determine if the raw JS object from the runtime is an out data object.
    let result_obj: v8::Local<v8::Object> = result_local.try_into().unwrap();
    let is_senc_out_data_key: v8::Local<v8::Value> =
        v8::String::new(&mut scope, "__is_senc_out_data").unwrap().into();
    if result_obj.has(&mut scope, is_senc_out_data_key).unwrap() {
        let out_type_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "out_type").unwrap().into();
        let out_type_local: v8::Local<v8::String> =
            result_obj.get(&mut scope, out_type_key).unwrap().try_into().unwrap();
        let out_type_str: &str = &out_type_local.to_rust_string_lossy(&mut scope);
        match out_type_str {
            "yaml" => { out_type = OutputType::YAML; },
            "" | "json" => {},  // Use default
            s => { return Err(anyhow!("out_type {s} in OutData object is not supported")) },
        }

        let out_ext_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "out_ext").unwrap().into();
        let out_ext_local: v8::Local<v8::String> =
            result_obj.get(&mut scope, out_ext_key).unwrap().try_into().unwrap();
        out_ext = out_ext_local.to_rust_string_lossy(&mut scope);

        let out_data_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "data").unwrap().into();
        result_local = result_obj.get(&mut scope, out_data_key).unwrap().try_into().unwrap();
    }

    let data = match out_type {
        OutputType::JSON => {
            let deserialized_result =
                serde_v8::from_v8::<serde_json::Value>(&mut scope, result_local).unwrap();
            serde_json::to_string(&deserialized_result).unwrap().to_string()
        },
        OutputType::YAML => {
            let deserialized_result =
                serde_v8::from_v8::<serde_yaml::Value>(&mut scope, result_local).unwrap();
            serde_yaml::to_string(&deserialized_result).unwrap().to_string()
        },
    };
    return Ok(OutData { out_ext, data });
}

fn write_data(out_file_stem: &str, data: &OutData) -> Result<()> {
    let mut out_file_path_str = out_file_stem.to_owned();
    out_file_path_str.push_str(&data.out_ext);

    let out_file_path = PathBuf::from(out_file_path_str);
    let out_file_dir = out_file_path.parent().unwrap();
    fs::create_dir_all(out_file_dir)?;
    let mut f = fs::File::create(out_file_path)?;
    f.write_all(data.data.as_bytes())?;

    return Ok(());
}

// The TypeScript module loader.
// This will check to see if the file is a TypeScript file, and run those through swc to transpile
// to JS.
//
// TODO:
// - Implement caching so only files that changed run through transpile.
struct TsModuleLoader;

impl ModuleLoader for TsModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, error::AnyError> {
        resolve_import(specifier, referrer).map_err(|e| e.into())
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> std::pin::Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        async move {
            let path = module_specifier.to_file_path().unwrap();

            // Determine what the MediaType is (this is done based on the file
            // extension) and whether transpiling is required.
            let media_type = MediaType::from_path(&path);
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => panic!("Unknown extension {:?}", path.extension()),
            };

            // Read the file, transpile if necessary.
            let code = std::fs::read_to_string(&path)?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.to_string(),
                    text_info: SourceTextInfo::from_string(code),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })?;
                parsed.transpile(&Default::default())?.text
            } else {
                code
            };

            // Load and return module.
            let module = ModuleSource::new(
                module_type,
                FastString::from(code),
                &module_specifier,
            );
            Ok(module)
        }
        .boxed_local()
    }
}
