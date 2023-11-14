// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

export function main() {
  const state = `${path.relpath(__projectroot, __dirname)}/terraform.tfstate`;
  const mainf = path.relpath(__projectroot, __filename);
  return { state, mainf };
}
