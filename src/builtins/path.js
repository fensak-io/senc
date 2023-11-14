// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

// The path API. Makes available the following functions:
// - path.relpath

((globalThis) => {
  const core = Deno.core;
  globalThis.path = {
    relpath: (base, p) => {
      return core.ops.op_path_relpath(base, p);
    },
  };
})(globalThis);
