// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  return new senc.OutData({
    out_ext: ".yml",
    out_type: "yaml",
    out_prefix: "# this is a prefix\n",
    data: { this: "outfile.js" },
  })
}
