use clap::Parser;
use std::rc::Rc;
use anyhow::{Context, Result};
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::futures::FutureExt;
use deno_core::Snapshot;

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/SENC_SNAPSHOT.bin"));

#[derive(Parser)]
struct Cli {
  path: std::path::PathBuf,
}

struct TsModuleLoader;

impl deno_core::ModuleLoader for TsModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _kind: deno_core::ResolutionKind,
  ) -> Result<deno_core::ModuleSpecifier, deno_core::error::AnyError> {
    deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
  }

  fn load(
    &self,
    module_specifier: &deno_core::ModuleSpecifier,
    _maybe_referrer: Option<&deno_core::ModuleSpecifier>,
    _is_dyn_import: bool,
  ) -> std::pin::Pin<Box<deno_core::ModuleSourceFuture>> {
    let module_specifier = module_specifier.clone();
    async move {
      let path = module_specifier.to_file_path().unwrap();

      // Determine what the MediaType is (this is done based on the file
      // extension) and whether transpiling is required.
      let media_type = MediaType::from_path(&path);
      let (module_type, should_transpile) = match media_type {
        MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
          (deno_core::ModuleType::JavaScript, false)
        }
        MediaType::Jsx => (deno_core::ModuleType::JavaScript, true),
        MediaType::TypeScript
        | MediaType::Mts
        | MediaType::Cts
        | MediaType::Dts
        | MediaType::Dmts
        | MediaType::Dcts
        | MediaType::Tsx => (deno_core::ModuleType::JavaScript, true),
        MediaType::Json => (deno_core::ModuleType::Json, false),
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
      let module = deno_core::ModuleSource::new(
        module_type,
        deno_core::FastString::from(code),
        &module_specifier,
      );
      Ok(module)
    }
    .boxed_local()
  }
}

async fn run_js(file_path: &str) -> Result<()> {
  let builtin_consolejs = deno_core::FastString::from(String::from(include_str!("./builtins/console.js")));

  let main_module = deno_core::resolve_path(file_path, std::env::current_dir()?.as_path())?;
  let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
      module_loader: Some(Rc::new(TsModuleLoader)),
      startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
      ..Default::default()
  });
  js_runtime.execute_script("[senc:builtins/console.js]",  builtin_consolejs).unwrap();

  let mod_id = js_runtime.load_main_module(&main_module, None).await?;
  let result = js_runtime.mod_evaluate(mod_id);
  js_runtime.run_event_loop(false).await?;
  result.await?
}

fn main() -> Result<()> {
  let args = Cli::parse();
  let fpath_str = args.path.to_str()
      .with_context(|| format!("could not read path string `{}`", args.path.display()))?;

  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  runtime.block_on(run_js(fpath_str))
      .with_context(|| format!("could not execute javascript file `{}`", args.path.display()))?;

  return Ok(());
}
