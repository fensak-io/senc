// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::io::Write;
use std::path;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use deno_core::*;

use crate::module_loader;

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

// Initialize the v8 platform. This should be called in the main thread before any subthreads are
// launched.
pub fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    JsRuntime::init_platform(Some(platform));
}

// Run the javascript or typescript file available at the given file path through the Deno runtime.
pub async fn run_js(node_modules_dir: Option<path::PathBuf>, req: &RunRequest) -> Result<()> {
    let mut js_runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(module_loader::TsModuleLoader::new(
            node_modules_dir,
        ))),
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        ..Default::default()
    });

    let mod_id = load_main_module(&mut js_runtime, &req.in_file).await?;
    let main_fn = load_main_fn(&mut js_runtime, mod_id).unwrap();
    let result = js_runtime.call_and_await(&main_fn).await?;
    let out_data = load_result(&mut js_runtime, result).unwrap();
    return write_data(&req.out_file_stem, &out_data);
}

async fn load_main_module(js_runtime: &mut JsRuntime, file_path: &str) -> Result<usize> {
    let main_module = resolve_path(file_path, std::env::current_dir()?.as_path())?;
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(false).await?;
    result.await?.unwrap();
    return Ok(mod_id);
}

fn load_main_fn(js_runtime: &mut JsRuntime, mod_id: usize) -> Result<v8::Global<v8::Function>> {
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

fn load_result(js_runtime: &mut JsRuntime, result: v8::Global<v8::Value>) -> Result<OutData> {
    let mut scope = &mut js_runtime.handle_scope();
    let mut result_local = v8::Local::new(&mut scope, result);

    let mut out_ext = ".json".to_string();
    let mut out_type = OutputType::JSON;

    // Determine if the raw JS object from the runtime is an out data object.
    let result_obj: v8::Local<v8::Object> = result_local.try_into().unwrap();
    let is_senc_out_data_key: v8::Local<v8::Value> =
        v8::String::new(&mut scope, "__is_senc_out_data")
            .unwrap()
            .into();
    if result_obj.has(&mut scope, is_senc_out_data_key).unwrap() {
        let out_type_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "out_type").unwrap().into();
        let out_type_local: v8::Local<v8::String> = result_obj
            .get(&mut scope, out_type_key)
            .unwrap()
            .try_into()
            .unwrap();
        let out_type_str: &str = &out_type_local.to_rust_string_lossy(&mut scope);
        match out_type_str {
            "yaml" => {
                out_type = OutputType::YAML;
            }
            "" | "json" => {} // Use default
            s => return Err(anyhow!("out_type {s} in OutData object is not supported")),
        }

        let out_ext_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "out_ext").unwrap().into();
        let out_ext_local: v8::Local<v8::String> = result_obj
            .get(&mut scope, out_ext_key)
            .unwrap()
            .try_into()
            .unwrap();
        out_ext = out_ext_local.to_rust_string_lossy(&mut scope);

        let out_data_key: v8::Local<v8::Value> =
            v8::String::new(&mut scope, "data").unwrap().into();
        result_local = result_obj
            .get(&mut scope, out_data_key)
            .unwrap()
            .try_into()
            .unwrap();
    }

    let data = match out_type {
        OutputType::JSON => {
            let deserialized_result =
                serde_v8::from_v8::<serde_json::Value>(&mut scope, result_local).unwrap();
            serde_json::to_string(&deserialized_result)
                .unwrap()
                .to_string()
        }
        OutputType::YAML => {
            let deserialized_result =
                serde_v8::from_v8::<serde_yaml::Value>(&mut scope, result_local).unwrap();
            serde_yaml::to_string(&deserialized_result)
                .unwrap()
                .to_string()
        }
    };
    return Ok(OutData { out_ext, data });
}

fn write_data(out_file_stem: &str, data: &OutData) -> Result<()> {
    let mut out_file_path_str = out_file_stem.to_owned();
    out_file_path_str.push_str(&data.out_ext);

    let out_file_path = path::PathBuf::from(out_file_path_str);
    let out_file_dir = out_file_path.parent().unwrap();
    fs::create_dir_all(out_file_dir)?;
    let mut f = fs::File::create(out_file_path)?;
    f.write_all(data.data.as_bytes())?;

    return Ok(());
}
