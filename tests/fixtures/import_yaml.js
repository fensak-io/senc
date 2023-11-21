// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import cfg from "./someconfig.yaml" with { type: "json" };

export function main() {
  return cfg;
}
