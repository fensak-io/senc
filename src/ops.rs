// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::path;

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

#[op2]
#[string]
pub fn op_path_relpath(
    #[string] base_str: &str,
    #[string] p_str: &str,
) -> Result<String, error::AnyError> {
    let p = path::Path::new(p_str);
    let relp = p.strip_prefix(base_str)?;
    Ok(relp.to_string_lossy().to_string())
}

#[op2(fast)]
#[string]
pub fn op_hcl_parse(#[string] hcl: &str) -> Result<(), error::AnyError> {
    //let hcl_json = hcl_to_json(hcl)?;
    let hcl_json = hcl::from_str(hcl)?;
    Ok(hcl_json)
}
