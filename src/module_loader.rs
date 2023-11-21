// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path;
use std::pin;

use anyhow::{anyhow, Result as AnyhowResult};
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::futures::FutureExt;
use deno_core::*;
use log::*;

// The transpile type. Determines how the code should be transpiled before loading.
enum TranspileType {
    No,         // No transpilation.
    Typescript, // Transpile typescript files to javascript.
    YAML,       // Transpile yaml files to json.
}

// The TypeScript module loader.
// This will check to see if the file is a TypeScript file, and run those through swc to transpile
// to JS.
//
// TODO:
// - Implement caching so only files that changed run through transpile.
pub struct TsModuleLoader {
    node_modules_dir: Option<path::PathBuf>,
}

impl TsModuleLoader {
    pub fn new(node_modules_dir: Option<path::PathBuf>) -> TsModuleLoader {
        TsModuleLoader { node_modules_dir }
    }

    // This resolves the given specifier as a node_modules module. Note that if the module loader
    // could not find any node modules dir in the parent tree, then this will return an error for
    // the specifier.
    fn resolve_node_module_import(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
        original_result: Result<ModuleSpecifier, error::AnyError>,
    ) -> Result<ModuleSpecifier, error::AnyError> {
        let node_modules_path = match &self.node_modules_dir {
            None => {
                error!("no node modules dir");
                return original_result;
            }
            Some(p) => p,
        };

        let new_specifier_path = find_node_module_specifier(node_modules_path, specifier)?;
        let new_specifier = new_specifier_path.to_str().unwrap();

        resolve_import(new_specifier, referrer).map_err(|e| e.into())
    }
}

impl ModuleLoader for TsModuleLoader {
    // This will handle imports exactly the same as Deno, handling URLs and relative imports.
    // For all other imports, this will assume it is available in the node_modules directory.
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, error::AnyError> {
        let res: Result<ModuleSpecifier, error::AnyError> =
            resolve_import(specifier, referrer).map_err(|e| e.into());
        match &res {
            Err(e) => match e.downcast_ref::<ModuleResolutionError>() {
                Some(ModuleResolutionError::ImportPrefixMissing(_, _)) => {
                    self.resolve_node_module_import(specifier, referrer, kind, res)
                }
                Some(_) => res,
                None => res,
            },
            _ => res,
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> pin::Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        async move {
            let orig_path = module_specifier.to_file_path().unwrap();

            // If there is no extension, assume .ts or .js (in that order) depending on if the path
            // exists.
            let path = match orig_path.extension() {
                Some(_) => orig_path.clone(),
                None => {
                    let mut maybe_ts = orig_path.clone();
                    maybe_ts.set_extension("ts");
                    let mut maybe_js = orig_path.clone();
                    maybe_js.set_extension("js");
                    if maybe_ts.is_file() {
                        maybe_ts
                    } else if maybe_js.is_file() {
                        maybe_js
                    } else {
                        return Err(anyhow!("{} not found", orig_path.to_string_lossy()));
                    }
                }
            };

            // Determine what the MediaType is (this is done based on the file
            // extension) and whether transpiling is required.
            let media_type = MediaType::from_path(&path);
            let (module_type, transpile_type) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, TranspileType::No)
                }
                MediaType::Jsx => (ModuleType::JavaScript, TranspileType::Typescript),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, TranspileType::Typescript),
                MediaType::Json => (ModuleType::Json, TranspileType::No),
                _ => {
                    let e = Err(anyhow!("Unknown extension {:?}", path.extension()));
                    match orig_path.extension() {
                        Some(os_str) => {
                            let lowercase_str = os_str.to_str().map(|s| s.to_lowercase());
                            match lowercase_str.as_deref() {
                                Some("yaml") | Some("yml") => {
                                    (ModuleType::Json, TranspileType::YAML)
                                }
                                _ => return e,
                            }
                        }
                        _ => return e,
                    }
                }
            };

            // Read the file, transpile if necessary.
            let code = fs::read_to_string(&path)?;
            let code = match transpile_type {
                TranspileType::No => code,
                TranspileType::Typescript => {
                    let parsed = deno_ast::parse_module(ParseParams {
                        specifier: module_specifier.to_string(),
                        text_info: SourceTextInfo::from_string(code),
                        media_type,
                        capture_tokens: false,
                        scope_analysis: false,
                        maybe_syntax: None,
                    })?;
                    parsed.transpile(&Default::default())?.text
                }
                TranspileType::YAML => {
                    let parsed: serde_json::Value = serde_yaml::from_str(&code)?;
                    serde_json::to_string(&parsed)?
                }
            };

            // Load and return module.
            let module = ModuleSource::new(module_type, FastString::from(code), &module_specifier);
            Ok(module)
        }
        .boxed_local()
    }
}

fn find_node_module_specifier(
    node_modules_dir: &path::PathBuf,
    specifier: &str,
) -> AnyhowResult<path::PathBuf> {
    let specifier_path = node_modules_dir.join(path::PathBuf::from(specifier));
    if specifier_path.is_file() {
        return Ok(fs::canonicalize(specifier_path)?);
    }

    let package_json_path = specifier_path.join("package.json");
    if !package_json_path.is_file() {
        return Err(anyhow!(
            "node package {} does not have a package.json file",
            specifier
        ));
    }

    let package_json_raw = fs::read_to_string(package_json_path)?;
    let package_json: serde_json::Value = serde_json::from_str(&package_json_raw)?;
    if package_json["module"] == serde_json::Value::Null {
        return Err(anyhow!("node package {} does not have ESM root", specifier));
    }

    let specifier_root_path = specifier_path.join(package_json["module"].as_str().unwrap());
    return Ok(fs::canonicalize(specifier_root_path)?);
}
