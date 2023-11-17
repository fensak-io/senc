// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  const d = { foo: "bar", fizz: 42, obj: { msg: "hello world" } };
  return new senc.OutData({
    out_ext: ".yml",
    out_type: "yaml",
    data: d,
  });
}
