// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import { foo } from "./foo.js";
import { find } from "lodash-es";

console.log("Hello world");
console.error("Boom!");

export function main() {
  const f = find(foo(), (i) => {
    return i.foo === "bar";
  })
  return new senc.OutData({
    out_ext: ".yml",
    out_type: "yaml",
    data: f,
  });
}
