// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

interface Foo {
  foo: string;
  fizz: number;
  obj: { msg: string };
};
const foo: Foo = {
  foo: "bar",
  fizz: 42,
  obj: {
    msg: "hello world",
  },
};

export function main(): Foo {
  return foo;
}
