// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

const cfg = await senc.import_hcl(`${__dirname}/someconfig.hcl`);

export function main() {
  return cfg;
}
