// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

mod engine;
mod files;
mod threadpool;

use std::fs;
use std::path;
use std::process;
use std::sync::{atomic, Arc};

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
    let projectroot = fs::canonicalize(&args.projectroot).unwrap();
    let outdir = match fs::canonicalize(&args.outdir) {
        Ok(d) => d,
        Err(_e) => {
            fs::create_dir_all(&args.outdir)?;
            fs::canonicalize(&args.outdir).unwrap()
        }
    };

    engine::init_v8();

    let requests = files::get_run_requests_from_path(&fpath, &outdir, &projectroot)
        .with_context(|| format!("could not collect files to execute"))?;

    let has_quit = Arc::new(atomic::AtomicBool::new(false));
    let mut pool = threadpool::ThreadPool::new(args.parallelism, has_quit.clone());
    let hq = has_quit.clone();
    ctrlc::set_handler(move || {
        if hq.load(atomic::Ordering::SeqCst) {
            eprintln!("Received second SIGINT. Shutting down immediately.");
            process::exit(1);
        }

        eprintln!("Shutting down gracefully...");
        hq.store(true, atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    for r in requests {
        pool.run(r);
    }
    pool.wait()
        .with_context(|| format!("could not run all files"))?;

    return Ok(());
}
