// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0
//
// Build script to create a compiled snapshot of the builtin javascript files to improve startup
// times. This will:
// - Load the builtin javascript functions.
// - Compile the builtin files into a snapshot.
// - Store the snapshot in the output directory so that it will be embedded into the senc CLI at
//   compile time.

use std::env;
use std::path::PathBuf;

use deno_core::extension;
use deno_core::snapshot_util::{create_snapshot, CreateSnapshotOptions};

extension!(
  builtins,
  // TODO
  // Make dynamic so it uses all files in builtins
  js = [ dir "src/builtins", "console.js", "path.js", "senc.js" ],
  docs = "Built in functions for senc.",
);

fn main() {
    // Build the file path to the snapshot.
    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("SENC_SNAPSHOT.bin");

    // Create the snapshot.
    let _snapshot = create_snapshot(CreateSnapshotOptions {
        cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
        snapshot_path,
        startup_snapshot: None,
        skip_op_registration: false,
        extensions: vec![builtins::init_ops_and_esm()],
        compression_cb: None,
        with_runtime_cb: None,
    });

    println!("cargo:rerun-if-changed=build.rs");
    // TODO
    // Make dynamic so it uses all files in builtins
    println!("cargo:rerun-if-changed=src/builtins/console.js");
    println!("cargo:rerun-if-changed=src/builtins/path.js");
    println!("cargo:rerun-if-changed=src/builtins/senc.js");
}
