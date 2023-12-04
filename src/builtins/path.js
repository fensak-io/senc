// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

// The path API. Makes available the following functions:
// - path.rel

((globalThis) => {
  const core = Deno.core;
  globalThis.path = {
    rel: (base, p) => {
      return core.ops.op_path_relpath(base, p);
    },
  };
})(globalThis);
