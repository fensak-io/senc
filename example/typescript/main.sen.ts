// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import { TerraformConfig } from "@fensak-io/senc-tfcore";
import { NullResource } from "@fensak-io/senc-tfnull";

export function main(): TerraformConfig {
  const tf = new TerraformConfig();
  const nr = new NullResource(
    { triggers: { foo: "world" } },
    { count: 5 },
  );
  tf.addResource("foo", nr);
  return tf;
}
