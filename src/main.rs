// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

mod files;
mod engine;
mod threadpool;

use anyhow::{Context, Result};
use clap::Parser;

// senc is a hermetic TypeScript interpreter for generating Infrastructure as Code (IaC).
//
// Use a familiar, type-safe programming language to define and provision infrastructure, with
// protections that make your code easy to debug and test.
#[derive(Parser)]
struct Cli {
  // The path to a .sen file or folder containing .sen files for generating IaC.
  pub path: std::path::PathBuf,

  // The number of files to process in parallel. This corresponds to the number of threads to
  // spawn.
  //
  // When 0, defaults to the number of cores available on the machine.
  #[clap(
    short='p',
    long,
    default_value_t=0,
    help="The number of files to process in parallel.",
  )]
  pub parallelism: usize,
}

fn main() -> Result<()> {
  let args = Cli::parse();

  let requests = files::get_run_requests_from_path(args.path.as_path())
    .with_context(|| format!("could not collect files to execute"))?;

  let pool = threadpool::ThreadPool::new(args.parallelism);

  for r in requests {
    pool.execute(r);
  }

  return Ok(());
}
