// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

mod engine;

use anyhow::Result;
use clap::Parser;

// senc is a hermetic TypeScript interpreter for generating Infrastructure as Code (IaC).
//
// Use a familiar, type-safe programming language to define and provision infrastructure, with
// protections that make your code easy to debug and test.
#[derive(Parser)]
struct Cli {
  // The path to a .sen file or folder containing .sen files for generating IaC.
  pub path: String,
}

fn main() -> Result<()> {
  let args = Cli::parse();
  return engine::start_thread(&args.path);
}
