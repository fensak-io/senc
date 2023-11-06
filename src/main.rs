// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

mod engine;
mod files;
mod threadpool;

use std::fs;
use std::path;

use anyhow::{Context, Result};
use clap::Parser;

// senc is a hermetic TypeScript interpreter for generating Infrastructure as Code (IaC).
//
// Use a familiar, type-safe programming language to define and provision infrastructure, with
// protections that make your code easy to debug and test.
#[derive(Parser)]
struct Cli {
    // The path to a .sen file or folder containing .sen files for generating IaC.
    pub path: path::PathBuf,

    // The path to a directory where the IaC files should be generated.
    #[clap(
    short='o',
    long,
    default_value_t=String::from("generated"),
    help="The path to a directory where the IaC files should be generated.",
  )]
    pub outdir: String,

    // The root directory of the project.
    #[clap(
    short='r',
    long,
    default_value_t=String::from("."),
    help="The root directory of the project.",
  )]
    pub projectroot: String,

    // The number of files to process in parallel. This corresponds to the number of threads to
    // spawn.
    //
    // When 0, defaults to the number of cores available on the machine.
    #[clap(
        short = 'p',
        long,
        default_value_t = 0,
        help = "The number of files to process in parallel."
    )]
    pub parallelism: usize,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let fpath = fs::canonicalize(&args.path).unwrap();
    let outdir = fs::canonicalize(&args.outdir).unwrap();
    let projectroot = fs::canonicalize(&args.projectroot).unwrap();

    let requests = files::get_run_requests_from_path(&fpath, &outdir, &projectroot)
        .with_context(|| format!("could not collect files to execute"))?;

    let pool = threadpool::ThreadPool::new(args.parallelism);

    for r in requests {
        pool.execute(r);
    }

    return Ok(());
}
