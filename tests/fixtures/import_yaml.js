// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

const cfg = await senc.import_yaml(`${__dirname}/someconfig.yaml`);

export function main() {
  return cfg;
}
