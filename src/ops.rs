// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::path;

use deno_core::*;
use deno_error::JsErrorBox;
use log::*;

#[op2(fast)]
pub fn op_log_trace(#[string] msg: &str) -> Result<(), JsErrorBox> {
    trace!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_debug(#[string] msg: &str) -> Result<(), JsErrorBox> {
    debug!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_info(#[string] msg: &str) -> Result<(), JsErrorBox> {
    info!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_warn(#[string] msg: &str) -> Result<(), JsErrorBox> {
    warn!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_error(#[string] msg: &str) -> Result<(), JsErrorBox> {
    error!("{msg}");
    Ok(())
}

#[op2]
#[string]
pub fn op_path_relpath(
    #[string] base_str: &str,
    #[string] p_str: &str,
) -> Result<String, JsErrorBox> {
    let p = path::Path::new(p_str);
    let relp = p
        .strip_prefix(base_str)
        .map_err(|err| JsErrorBox::generic(err.to_string()))?;
    Ok(relp.to_string_lossy().to_string())
}
