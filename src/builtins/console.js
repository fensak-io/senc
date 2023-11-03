// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0
//
// TODO
// Switch to ESM syntax


// The console API. Makes available the following functions:
// - console.log
// - console.error

((globalThis) => {
  const core = Deno.core;

  function argsToMessage(...args) {
    return args.map((arg) => JSON.stringify(arg)).join(" ");
  }

  globalThis.console = {
    log: (...args) => {
      core.print(`[out]: ${argsToMessage(...args)}\n`, false);
    },
    error: (...args) => {
      core.print(`[err]: ${argsToMessage(...args)}\n`, true);
    },
  };
})(globalThis);
