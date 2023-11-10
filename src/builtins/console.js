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
      core.ops.op_log_info(argsToMessage(...args));
    },
    trace: (...args) => {
      core.ops.op_log_trace(argsToMessage(...args));
    },
    debug: (...args) => {
      core.ops.op_log_debug(argsToMessage(...args));
    },
    info: (...args) => {
      core.ops.op_log_info(argsToMessage(...args));
    },
    warn: (...args) => {
      core.ops.op_log_warn(argsToMessage(...args));
    },
    error: (...args) => {
      core.ops.op_log_error(argsToMessage(...args));
    },
  };
})(globalThis);
