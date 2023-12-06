// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  return new senc.OutData({
    schema_path: "schema.json",
    data: {
      productId: 5,
      shouldNotHave: true,
    },
  });
}
