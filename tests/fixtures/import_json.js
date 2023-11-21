// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import cfg from "./someconfig.json" with { type: "json" };

export function main() {
  return cfg;
}
