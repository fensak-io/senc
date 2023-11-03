use std::env;
use std::path::PathBuf;
use deno_core::extension;
use deno_core::snapshot_util::{create_snapshot, CreateSnapshotOptions};

extension!(
  builtins,
  // TODO
  // Make dynamic so it uses all files in builtins
  js = [ dir "src/builtins", "console.js" ],
  docs = "Built in functions for senc.",
);

fn main() {
  // Build the file path to the snapshot.
  let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  let snapshot_path = o.join("SENC_SNAPSHOT.bin");

  // Create the snapshot.
  let _snapshot = create_snapshot(CreateSnapshotOptions {
    cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
    snapshot_path: snapshot_path,
    startup_snapshot: None,
    extensions: vec![builtins::init_ops_and_esm()],
    compression_cb: None,
    with_runtime_cb: None,
  });

  println!("cargo:rerun-if-changed=build.rs");
  // TODO
  // Make dynamic so it uses all files in builtins
  println!("cargo:rerun-if-changed=src/builtins/index.js");
  println!("cargo:rerun-if-changed=src/builtins/console.js");
}
