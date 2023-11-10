// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import { find } from "lodash-es";

const foo = [
  {
    foo: "bar",
    cfg: false,
  },
  {
    foo: "foo",
    cfg: true,
  },
];

export function main() {
  const f = find(foo, (i) => {
    return i.foo === "bar";
  });
  return f;
}
