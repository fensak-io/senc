// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  return new senc.OutData({
    out_ext: ".hcl",
    out_type: "hcl",
    out_prefix: "### this is a prefix\n#\n###\n",
    data: {
      "some_attr": {
        "foo": [1, 2],
        "bar": true
      },
      "some_block": {
        "some_block_label": {
          "attr": "value"
        }
      }
    },
  })
}
