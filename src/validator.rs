// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::io;
use std::path;

use anyhow::{anyhow, Result};
use jsonschema::{Draft, JSONSchema};

pub trait DataSchema {
    fn validate(&self, data: &serde_json::Value) -> Result<()>;
}

pub struct DataJSONSchema {
    schema: JSONSchema,
}

impl DataSchema for DataJSONSchema {
    fn validate(&self, data: &serde_json::Value) -> Result<()> {
        match self.schema.validate(data) {
            Err(errs) => {
                let mut err_strs = Vec::new();
                for err in errs {
                    let instance_path_str = err.instance_path.to_string();
                    let err_str = if instance_path_str == "" {
                        format!("[.] {}", err).to_string()
                    } else {
                        format!("[{}] {}\n", instance_path_str, err).to_string()
                    };
                    err_strs.push(err_str);
                }
                Err(anyhow!(err_strs.join("\n")))
            }
            Ok(result) => Ok(result),
        }
    }
}

pub fn new_from_path(schema_path: &path::Path) -> Result<impl DataSchema> {
    let schema_file = fs::File::open(schema_path)?;
    let schema_reader = io::BufReader::new(schema_file);
    let raw_schema: serde_json::Value = serde_json::from_reader(schema_reader)?;

    let maybe_jsonschema: Result<JSONSchema, _> = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&raw_schema);
    match maybe_jsonschema {
        Ok(jsonschema) => {
            return Ok(DataJSONSchema { schema: jsonschema });
        }
        Err(err) => {
            return Err(anyhow!(
                "Could not load schema {}: {}",
                schema_path.to_string_lossy(),
                err
            ));
        }
    };
}
