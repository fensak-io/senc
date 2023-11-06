// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::path;
use std::fs;

use anyhow::Result;
use walkdir::WalkDir;
use regex::Regex;

use crate::engine;

pub fn get_run_requests_from_path(file_path: &path::Path) -> Result<Vec<engine::RunRequest>> {
  let meta = match fs::metadata(file_path) {
    Ok(r) => { r },
    Err(error) => { return Err(error.into()) },
  };

  if !meta.is_dir() {
    let in_file = file_path.to_str().unwrap().to_string();
    let out_file = "".to_string();
    let mut reqs = Vec::with_capacity(1);
    reqs.push(engine::RunRequest { in_file, out_file });
    return Ok(reqs);
  }

  let find_sen_re = Regex::new(r".+\.sen\.(js|ts)$").unwrap();

  let mut reqs = Vec::new();
  for entry in WalkDir::new(file_path)
      .into_iter()
      .filter_map(Result::ok)
      .filter(|e| !e.file_type().is_dir()) {
    let fname_str = String::from(entry.file_name().to_string_lossy());
    if !find_sen_re.is_match(&fname_str) {
      continue;
    }

    let in_file = String::from(entry.path().to_string_lossy());
    let out_file = "".to_string();
    reqs.push(engine::RunRequest { in_file, out_file });
  }
  return Ok(reqs);
}
