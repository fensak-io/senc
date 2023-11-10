// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use deno_core::*;
use log::*;

#[op2(fast)]
pub fn op_log_trace(#[string] msg: &str) -> Result<(), error::AnyError> {
    trace!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_debug(#[string] msg: &str) -> Result<(), error::AnyError> {
    debug!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_info(#[string] msg: &str) -> Result<(), error::AnyError> {
    info!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_warn(#[string] msg: &str) -> Result<(), error::AnyError> {
    warn!("{msg}");
    Ok(())
}

#[op2(fast)]
pub fn op_log_error(#[string] msg: &str) -> Result<(), error::AnyError> {
    error!("{msg}");
    Ok(())
}
