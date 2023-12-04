// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

const cfg = await senc.import_json(`${__dirname}/someconfig.json`);

export function main() {
  return cfg;
}
