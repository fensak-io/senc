// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  const l = new senc.OutDataArray();
  const d1 = new senc.OutData({
    out_ext: ".yml",
    out_type: "yaml",
    data: { this: "outfile.js" },
  });
  l.push(d1);
  const d2 = new senc.OutData({
    out_ext: ".json",
    out_type: "json",
    data: { this: "outfile.js" },
  });
  l.push(d2);
  const d3 = new senc.OutData({
    out_path: "ex/ample/foo.json",
    out_type: "json",
    data: { this: "ex/ample/outfile.js" },
  });
  l.push(d3);
  return l;
}
