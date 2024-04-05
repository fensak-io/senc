// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0
//
// TODO
// Switch to ESM syntax


// The HCL API. Makes the following functions available:
// - hcl.parse

((globalThis) => {
    const core = Deno.core;
  
    // function argsToMessage(...args) {
    //   return args.map((arg) => JSON.stringify(arg)).join(" ");
    // }

    globalThis.hcl = {
      parse: (text) => {
        return core.ops.op_hcl_parse(text);
      },
    };
  })(globalThis);
  