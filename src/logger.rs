// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::io::Write;

use log::*;

pub fn init(level: &str, no_color: bool) {
    let mut logger_env = env_logger::Env::new()
        .filter("SENC_LOG")
        .write_style("SENC_LOG_STYLE")
        .default_filter_or(level);
    if no_color {
        logger_env = logger_env.default_write_style_or("never");
    }

    env_logger::Builder::from_env(logger_env)
        .format(|buf, record| {
            let mut style = buf.style();
            let level = record.level();
            match level {
                Level::Error => {
                    style.set_color(env_logger::fmt::Color::Red).set_bold(true);
                }
                Level::Warn => {
                    style.set_color(env_logger::fmt::Color::Yellow);
                }
                Level::Debug => {
                    style.set_dimmed(true);
                }
                _ => {}
            }

            writeln!(
                buf,
                "{}: {}",
                style.value(record.level()),
                style.value(record.args())
            )
        })
        .init();
}
