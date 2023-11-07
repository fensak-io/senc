// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

console.log("Hello world");
console.error("Boom!");

export function main() {
  return new senc.OutData(".yml", "yaml", { cfg: true });
}
