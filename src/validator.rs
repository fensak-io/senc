// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::io;
use std::path;

use anyhow::{anyhow, Result};
use jsonschema::{Draft, Validator};

pub trait DataSchema {
    fn validate(&self, data: &serde_json::Value) -> Result<()>;
}

pub struct DataJSONSchema {
    schema: Validator,
}

impl DataSchema for DataJSONSchema {
    fn validate(&self, data: &serde_json::Value) -> Result<()> {
        match self.schema.validate(data) {
            Err(err) => {
                let instance_path_str = err.instance_path().to_string();
                let err_str = if instance_path_str.is_empty() {
                    format!("[.] {err}")
                } else {
                    format!("[{instance_path_str}] {err}")
                };
                Err(anyhow!(err_str))
            }
            Ok(result) => Ok(result),
        }
    }
}

pub fn new_from_path(schema_path: &path::Path) -> Result<impl DataSchema> {
    let schema_file = fs::File::open(schema_path)?;
    let schema_reader = io::BufReader::new(schema_file);
    let raw_schema: serde_json::Value = serde_json::from_reader(schema_reader)?;

    let maybe_jsonschema: Result<Validator, _> = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(&raw_schema);
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
