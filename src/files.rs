// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use walkdir::WalkDir;

use crate::engine;

lazy_static! {
    static ref FIND_SEN_RE: Regex = Regex::new(r".+\.sen\.(js|ts)$").unwrap();
}

// Collects the list of files that need to be run through Deno by senc.
//
// If the path is a file, only that file is run through. If the file is a directory, this will
// recursively walk through the directory looking for files that end in .sen.js or .sen.ts.
pub fn get_run_requests_from_path(
    file_path: &path::Path,
    outdir: &path::Path,
    projectroot: &path::Path,
) -> Result<Vec<engine::RunRequest>> {
    if let Err(error) = assert_file_path_in_projectroot(file_path, projectroot) {
        return Err(error.into());
    }

    let meta = match fs::metadata(file_path) {
        Ok(r) => r,
        Err(error) => return Err(error.into()),
    };

    if meta.is_dir() {
        return run_requests_from_dir(file_path, outdir, projectroot);
    }
    return run_requests_from_file(file_path, outdir, projectroot);
}

fn run_requests_from_file(
    file_path: &path::Path,
    outdir: &path::Path,
    projectroot: &path::Path,
) -> Result<Vec<engine::RunRequest>> {
    let fpath_str = file_path.to_string_lossy();
    if !FIND_SEN_RE.is_match(&fpath_str) {
        return Err(anyhow!("{fpath_str} does not end with .sen.js or .sen.ts"));
    }

    let in_file = String::from(fpath_str);
    let out_file_stem = get_out_file_stem(file_path, outdir, projectroot).unwrap();

    let mut reqs = Vec::with_capacity(1);
    reqs.push(engine::RunRequest {
        in_file,
        out_file_stem,
    });
    return Ok(reqs);
}

fn run_requests_from_dir(
    file_path: &path::Path,
    outdir: &path::Path,
    projectroot: &path::Path,
) -> Result<Vec<engine::RunRequest>> {
    let mut reqs = Vec::new();
    for entry in WalkDir::new(file_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| FIND_SEN_RE.is_match(&e.file_name().to_string_lossy()))
    {
        let file_path = fs::canonicalize(entry.path()).unwrap();
        let in_file = String::from(file_path.to_string_lossy());
        let out_file_stem = get_out_file_stem(file_path.as_path(), outdir, projectroot).unwrap();
        reqs.push(engine::RunRequest {
            in_file,
            out_file_stem,
        });
    }
    return Ok(reqs);
}

fn assert_file_path_in_projectroot(file_path: &path::Path, projectroot: &path::Path) -> Result<()> {
    if file_path == projectroot {
        return Ok(());
    }
    let mut maybe_fp = file_path.parent();
    while maybe_fp != None {
        let fp = maybe_fp.unwrap();
        if fp == projectroot {
            return Ok(());
        }
        maybe_fp = fp.parent();
    }

    return Err(anyhow!(
        "{} is not in {}",
        file_path.to_string_lossy(),
        projectroot.to_string_lossy()
    ));
}

fn get_out_file_stem(
    file_path: &path::Path,
    outdir: &path::Path,
    projectroot: &path::Path,
) -> Result<String> {
    // Call file_stem twice since we want to drop both .js/.ts and .sen.
    let fname = path::Path::new(file_path.file_name().unwrap());
    let fname_stem = path::Path::new(fname.file_stem().unwrap())
        .file_stem()
        .unwrap();

    // Construct the output file stem
    let file_dir = file_path.parent().unwrap();
    let out_file_dir = outdir.join(file_dir.strip_prefix(projectroot).unwrap());
    return Ok(String::from(
        out_file_dir.join(fname_stem).to_string_lossy(),
    ));
}
