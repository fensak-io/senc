# senc

<p align="center">
  <a href="https://github.com/fensak-io/senc/releases/latest">
    <img alt="latest release" src="https://img.shields.io/github/v/release/fensak-io/senc?style=for-the-badge">
  </a>
  <a href="https://github.com/fensak-io/senc/blob/main/LICENSE">
    <img alt="LICENSE" src="https://img.shields.io/badge/LICENSE-MPL_2.0-orange?style=for-the-badge">
  </a>
</p>

[senc](https://docs.senc.sh) (seh-nn-see) is a [hermetic](https://bazel.build/basics/hermeticity)
[TypeScript](https://www.typescriptlang.org/) interpreter for generating config files. `senc` supports generating any
arbitrary JSON/YAML configurations, including:

- CI config, like `.circleci/config.yml` or `.github/workflows`.
- OpenTofu/Terraform configuration (in [JSON format](https://developer.hashicorp.com/terraform/language/syntax/json)).
- Kubernetes manifests.

Use a familiar, type-safe programming language to define and provision infrastructure, with protections that make your
code easy to debug and test.


<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
## Table of Contents

- [Installation](#installation)
  - [Release builds](#release-builds)
  - [From source](#from-source)
- [Usage](#usage)
  - [Rendering json objects](#rendering-json-objects)
  - [Customizing the rendered output](#customizing-the-rendered-output)
  - [Rendering multiple output files](#rendering-multiple-output-files)
- [Features](#features)
  - [Restricted features](#restricted-features)
  - [Builtin functions](#builtin-functions)
  - [Types for builtins](#types-for-builtins)
  - [NPM packages](#npm-packages)
  - [Validating output data](#validating-output-data)
  - [Type libraries](#type-libraries)
- [Technology](#technology)
- [FAQ](#faq)
  - [What is Hermeticity?](#what-is-hermeticity)
  - [Why `senc` over Pulumi or CDK?](#why-senc-over-pulumi-or-cdk)
  - [Why `senc` over Terraform / OpenTofu?](#why-senc-over-terraform--opentofu)
  - [Why the name `senc`?](#why-the-name-senc)
- [Similar tools](#similar-tools)
- [License](#license)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Installation

### Release builds

The easiest way to get started with `senc` is to download a pre-compiled binary for your platform from [the latest
release](https://github.com/fensak-io/senc/releases/latest) on GitHub.

You can unpack the release artifact and install it somewhere in your `PATH`. Once `senc` is available, you can call it
from the command line:

```
senc -o /path/to/output/dir /path/to/input/dir
```

### From source

`senc` should build on latest stable Rust version (probably on the oldest, but there is no MSRV policy provided).

1. Install Rust using `rustup` following instructions [here](https://rust-lang.github.io/rustup/installation/index.html).
1. Once you have the Rust toolchain with `cargo`, clone and run `senc`:

```
git clone https://github.com/fensak-io/senc.git
cd senc

# NOTE
# This is not strictly necessary, but if you wish to have sane versions in Cargo.toml, then you will want to work off
# the release branch.
git switch release

cargo run -- -o /path/to/output/dir /path/to/input/dir
```


## Usage

`senc` searches for files with the `.sen.ts` or `.sen.js` extension in the project directory to use as entrypoints for
generating JSON and YAML configuration files. The entrypoint can be written in JavaScript (ECMAScript 6+) or TypeScript.
Each `senc` entrypoint is expected to export a `main` function that returns the object to be rendered.

When running `senc` without configuration options, `senc` will render each entrypoint script as a `json` file that has
the same filename as the entrypoint in the output directory.

For example, consider the following tree:

```
.
├── in
│   └── myconfig.sen.ts
└── out
```

Assuming `myconfig.sen.ts` has a valid `main` function, running `senc` with the command `senc -o ./out ./in`
will produce the following:

```
.
├── in
│   └── myconfig.sen.ts
└── out
    └── myconfig.json
```

If the input directory has subfolders, the same tree will be replicated in the output directory, relative to the input
root. For example:

```
.
├── in
│   ├── myconfig.sen.ts
│   └── nested
│       └── subfolders
│           └── anotherconfig.sen.ts
└── out
```

Will render as:

```
.
├── in
│   ├── myconfig.sen.ts
│   └── nested
│       └── subfolders
│           └── anotherconfig.sen.ts
└── out
    ├── myconfig.json
    └── nested
        └── subfolders
            └── anotherconfig.json
```

### Rendering json objects

If you are rendering json configuration, then the `main` function can return the config as a raw object to be rendered.
For example, if your entrypoint had the following:

```typescript
export function main() {
  return {
    id: 5,
    msg: "hello world",
  };
}
```

The rendered JSON will be:

```json
{
  "id": 5,
  "msg": "hello world"
}
```

Note that the entrypoint is run through a TypeScript compiler and JavaScript runtime. This means that you have access to
most standard JavaScript operations when constructing the output object. For example:

```typescript
export function main() {
  const cfg = {
    id: 5,
    msg: "",
  };
  cfg.msg = "hello world";
  return cfg;
}
```

will render in the same way as the previous example.

Refer to section [Restricted features](#restricted-features) for information on what is NOT available in the runtime.

### Customizing the rendered output

Return a `senc.OutData` object instead of the raw data to customize the rendered output. The `senc.OutData` object tags
the output data with metadata that indicates to `senc` how you wish to render the output. For example, to render the
config data as `yaml`:

```
export function main() {
  const cfg = {
    id: 5,
    msg: "hello world",
  };
  return new senc.OutData({
    out_type: "yaml",
    data: cfg,
  });
}
```

This will render the config as YAML, with the `.yaml` extension:

```yaml
id: 5
msg: "hello world"
```

The constructor for `senc.OutData` supports the following options:

- `out_path`: The path of the output file, relative to the output dir. Only one of `out_path` or `out_ext` can be set.
- `out_ext`: The extension of the output file, including the preceding `.` (e.g., `.json`).
- `out_type`: The type of the output file. Either `json` or `yaml`.
- `out_prefix`: An optional string to prepend to the rendered file output. This is useful for adding comments, such as a
                license header.
- `schema_path`: An optional path to a schema file to use for validating the rendered data. The path is relative to the
                 directory of the entrypoint. Currently only supports [jsonschema](https://json-schema.org/).
- `data`: The data to render to the output file. This can be any JSON/YAML serializable object.

### Rendering multiple output files

A single entrypoint can render multiple output files. This is useful when you want to programmatically decide which
folders/files to render in the configuration output where having separate configuration files matter (e.g.,
Terraform/OpenTofu).

To render multiple output files, you need to return a `senc.OutDataArray` object, which is a special `senc.OutData`
array. The `OutDataArray` object supports all the standard `Array` functions. For example:

```typescript
export function main() {
  const l = new senc.OutDataArray();
  const d1 = new senc.OutData({
    out_path: "out.yml",
    out_type: "yaml",
    data: { msg: "hello world" },
  });
  l.push(d1);
  const d2 = new senc.OutData({
    out_path: "out.json",
    out_type: "json",
    data: { msg: "世界こんにちは" },
  });
  l.push(d2);
  return l;
}
```

This will render two files, `out.yml` and `out.json`, each with the following contents:

**out.yml**
```yaml
msg: "hello world"
```

**out.json**
```json
{
    "msg": "世界こんにちは"
}
```


## Features

### Restricted features

`senc` aims to be a [hermetic](https://bazel.build/basics/hermeticity) runtime, and thus most system related calls and
environment access is disabled in the runtime. Specifically, the following standard JavaScript features are missing:

- Network calls (e.g., `fetch` and `XMLHttpRequest`).
- Filesystem access (e.g., `fs`), except through imports.
- Environment access (e.g., `process.env`).
- Process access

Note that there may be more disabled features that are not specified above, so don't expect a feature to be available
just because it isn't mentioned. We strive to update and keep this list up to date, but as a young project there may be
some edge cases that we missed.

### Builtin functions

`senc` ships with a few builtin functions that are available for use:

**console**

Console API for logging to `stderr`. You can log with different logging levels, which will be hidden depending on the
`--loglevel` option in the CLI. The following functions are available: `console.trace`, `console.debug`, `console.info`,
`console.warn`, `console.error`, `console.log`.

Example:
```javascript
console.info("hello", "world")
// INFO: hello world
```

**path**

Path API for manipulating or constructing filesystem paths. This is useful for constructing the `out_path` attribute of
the `senc.OutData` object.

`path.rel(base, p)`: Returns the relative path from `base` to `p`. Joining the result to `base` will return `p`.

Example:
```javascript
const base = "/home/senc/example"
const p = "/home/senc/example/some/path/to/file.js"
const r = path.rel(base, p)
// r is "some/path/to/file.js"
```

**senc**

`senc` specific API. Exposes the following:

`senc.OutData` and `senc.OutDataArray`: Custom objects for customizing output behavior.

`senc.import_json`: Import the given file path as a JSON object. This equivalent to loading the file from disk and
parsing it using `JSON.parse`.

NOTE:
- The provided path must be an absolute path. Use `__dirname` to construct the import path.
- For security purposes, this only supports importing files in the project root as configured through the senc CLI.

```js
const cfg = await senc.import_json(`${__dirname}/someconfig.json`);
```

`senc.import_yaml`: Same functionality as `import_json`, only interprets the content as YAML as opposed to JSON.

**constants**

`senc` exposes a few constants in the global scope that are useful for constructing output paths:

- `__projectroot`: The absolute path to the project root directory.
- `__dirname`: The absolute path to the directory containing the script file.
- `__filename`: The absolute path to the script file.

### Types for builtins

Since the `senc` builtins are not standard to most JavaScript runtimes, you may get type errors when opening `senc`
entrypoints in your IDE in TypeScript. To fix this, you must install and configure the `senc-types` package. Refer to
the NPM package page for more details:

[@fensak-io/senc-types](https://www.npmjs.com/package/@fensak-io/senc-types)

### NPM packages

`senc` supports looking up imports in the `node_modules` directory, meaning that you can use `npm` packages in your
scripts. To use an `npm` package, install it like you normally would using your favorite package manager (`npm`, `pnpm`,
`yarn`, etc) and import it:

```javascript
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
```

Some caveats:
- Currently the runtime only supports ESM modules. Follow https://github.com/fensak-io/senc/issues/7 for updates on when
  CJS is made available.
- Currently the runtime only works with `npm` modules that have a `module` key specified. It currently does NOT support
  looking at the `exports` key.
- The runtime does not support importing a file in the package directly. Follow
  https://github.com/fensak-io/senc/issues/17 for updates on when this functionality is made available.

### Validating output data

`senc` has builtin support for validating output data with [jsonschema](https://json-schema.org/). You can store a
jsonschema configuration in your project root and link to the output using the `schema_path` property of `senc.OutData`.
When a schema is linked, `senc` will validate the output data against the schema and throw an error if the rendered
object does not match the schema.

### Type libraries

We publish various auto generated type libraries that can be useful. Here are the officially maintained type libraries
that we provide:

- CI Configuration Files: [senc-schemastore-ciconfig](https://github.com/fensak-io/senc-schemastore-ciconfig)
  ([NPM](https://www.npmjs.com/package/@fensak-io/senc-schemastore-ciconfig)).


## Technology

`senc` is built in [Rust](https://www.rust-lang.org/), and embeds [the Deno runtime](https://deno.com) for the
TypeScript runtime using the [deno_core crate](https://docs.rs/crate/deno_core/latest).


## Real world examples

Fensak uses `senc` to manage CI configurations. Check out the following examples where it is used:

- [senc CircleCI config](https://github.com/fensak-io/senc/tree/main/_ci)


## FAQ

### What is Hermeticity?

[Hermeticity](https://bazel.build/basics/hermeticity) is the concept of a fully isolated build system that ensures the
output of a computation is always the same for the same input, regardless of the runtime environment. This is a concept
popularized in tools like [Bazel](https://bazel.build/) and [Jsonnet](https://jsonnet.org/), where hermeticity allowed
these systems to be super fast by enabling parallelism and aggressive caching in the process.

Hermeticity also has benefits in reproducibility, where it makes it really easy to analyze failing builds since there is
no dynamicism in the failure. Reproducing a failing build locally is as easy as pulling down the input sources and
retrying the build.

`senc` is an almost-hermetic runtime for TypeScript. It is "almost" because it exposes some limited access to the
environment, namely access to the file system (for code modularization) and stdout/stderr. However, it does not give
any other environmental access (e.g., network calls, environment variables, etc).


### Why `senc` over Pulumi or CDK?

Using TypeScript to provision and manage infrastructure is not a new concept. Existing tools such as
[Pulumi](https://www.pulumi.com/) and [CDK](https://aws.amazon.com/cdk/) already give you the ability to write
infrastructure code in TypeScript and provision it directly without external dependencies. So why bother with an extra
compilation step?

The main reason for this is because all these tools turn general purpose programming languages into an abstraction on
top of an underlying language for managing infrastructure. For Pulumi, this is a proprietary representation implemented
by the engine, which then gets reflected into the actual infrastructure. For CDK, this is either CloudFormation or
Terraform.

The challenge with the existing tools is that they hide away the intricacies of the underlying representation, making it
really difficult to trace down bugs in your code. When something goes wrong, it is oftentimes a nightmare to determine
if an issue is caused by a bug in the cloud layer, a bug in the infrastructure representation layer, or a bug in the top
TypeScript layer.

Another issue is that both Pulumi and CDK do not limit users in the TypeScript layer. For the most part, you can do
anything in the TypeScript layer, including reaching out to AWS APIs to inspect existing infrastructure. The cost of
this freedom is that it makes it difficult to test and develop against this code, since now you need to stand up actual
infrastructure. Depending on your runtime, this can also add overhead to credentials management. For example, if you
were using Terraform Cloud (TFC), you would need to first compile your infrastructure using `cdktf synth`, and then have
TFC deploy the compiled down code. If you have network dependent code in the TypeScript layer, then you would need to
share your credentials with both the CI system running `cdktf synth`, and TFC, expanding the surface area.

You can always restrict your team from using these features and have the same effect. However, in practice, if there is
a way to do something, it will always be used.

`senc` addresses both of these concerns by using an explicit hermetic compilation process. `senc` does not directly
provision infrastructure, delegating that task to the underlying infrastructure representation (either Terraform/OpenTofu,
or Kubernetes). This has a few advantages:

- Because the infrastructure provisioning step is explicit, it's very easy to trace down if a bug is from the Terraform
  code or TypeScript code. You can either introspect the generated code, or try running it directly yourself.
- `senc` is a hermetic runtime, and thus there is no way to write code that depends on the environment. This means that:
    - You can easily troubleshoot failing builds by rerunning locally with the same source.
    - You can run the compilation step without any credentials. Only share the credentials with your provisioning
      pipeline.
    - Testing can be done solely through introspection of the generated code. A typical testing pipeline would:
        1. Run `senc` to generate the IaC.
        1. Run validation to ensure the generated code is sound (e.g., `terraform validate`).
        1. Run a contract checker like [OPA](https://www.openpolicyagent.org/) or [CUE](https://cuelang.org/) to ensure
           the specific settings are set.

- Since `senc` doesn't handle the provisioning aspect, you can natively integrate with any of the Terraform runtimes,
  such as Terraform Cloud, Spacelift, env0, or Terraform/OpenTofu workflows on GitHub Actions.


### Why `senc` over Terraform / OpenTofu?

`senc` allows you to use TypeScript to provision and manage infrastructure. Although it does not give you the full range
of power behind the general purpose programming language (due to the hermeticity), it does give you access to the
expressiveness of the underlying programming language. This should be much more familiar to anyone who has experience
with general purpose programming languages than a DSL like HCL.

`senc` does not limit you from features available to Terraform/OpenTofu. Since `senc` is a code generator at heart, as
long as you generate the necessary Terrraform/OpenTofu code, you can use any feature or construct available.

However, by using a higher level language to generate the underlying Terraform/OpenTofu code, it allows you to
workaround certain limitations of HCL, most notably:
- You can interpolate constructs that can not be dynamically interpolated in HCL (e.g.,
  [lifecycle](https://developer.hashicorp.com/terraform/language/meta-arguments/lifecycle#literal-values-only) and
  [backend](https://developer.hashicorp.com/terraform/language/settings/backends/configuration)).
- You can reuse blocks that typically can't be reused (e.g.,
  [provider](https://developer.hashicorp.com/terraform/language/modules/develop/providers)).


### Why the name `senc`?

`senc` (pronounced seh-nn-see) comes from the word 仙人 (sen-nin) in Japanese, which itself is derived from
仙 (Xian) in Chinese. 仙人 refers to an immortal wizard or sage that is living as a hermit, typically in the mountains.
Note that the 人 character means "person" or "human."

The `c` in `senc` on the other hand means "compiler."

Putting all this together, `senc` can be translated to mean "compiler that is a hermit," which seems fitting for a
hermetic compiler.


## Similar tools

There are many alternative configuration languages that can be converted to JSON:

- [tyson](https://github.com/jetpack-io/tyson)
- [dhall](https://dhall-lang.org/)
- [cue](https://cuelang.org/)
- [jsonnet](https://jsonnet.org/)
- [nickel](https://nickel-lang.org/)

Most of these require learning a new DSL that offer different advantages and tradeoffs. Depending on your needs, the
advantages of using a separate DSL may be more beneficial than the cost of familiarizing yourself with a new language.

The main advantage of using `senc` over these tools is that `senc` uses JavaScript and TypeScript as the implementation
language, allowing you to use something that may be more expressive and flexible than some of the DSLs.

> **Note on TySON**
>
> TySON is also a TypeScript based configuration generator, but has a few features that are missing, the biggest one
> being lack of support for NPM modules.

For IaC specifically, there is also the following:

- [cdktf](https://developer.hashicorp.com/terraform/cdktf) and [cdk8s](https://cdk8s.io/)
- [Pulumi](https://www.pulumi.com/)
- [kapitan](https://kapitan.dev/)

As mentioned above in the FAQ, the main differentiator of `senc` compared to these tools is that it focuses solely on
compilation and code generation, making it easy to adopt incrementally, or mix and match with current and future
IaC runtimes.


## Contributing

Refer to our [Contribution Guide](/CONTRIBUTING.md).


## License

```
SPDX-License-Identifier: MPL-2.0
```
