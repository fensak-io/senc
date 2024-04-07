<h1 align="center">senc core types</h1>

<p align="center">
  <a href="https://www.npmjs.com/package/@fensak-io/senc-types">
    <img alt="NPM" src="https://img.shields.io/npm/v/@fensak-io/senc-types.svg?style=for-the-badge">
  </a>
  <a href="https://github.com/fensak-io/senc/blob/main/LICENSE">
    <img alt="LICENSE" src="https://img.shields.io/github/license/fensak-io/senc?style=for-the-badge">
  </a>
  <a href="https://github.com/fensak-io/senc/releases/latest">
    <img alt="latest release" src="https://img.shields.io/github/v/release/fensak-io/senc?style=for-the-badge">
  </a>
</p>

`senc-types` contains the global type definitions for [senc](https://github.com/fensak-io/senc) scripts.

## Usage

```
npm add --save-dev @fensak-io/senc-types
```

In your `tsconfig.json`, you must either add `@fensak-io` to the `typeRoots` setting, or `@fensak-io/senc-types` to the
`types` option so that the global declarations are included in your package.

**with typeRoots**
```json5
{
  // ...
  "compilerOptions: {
    // ...
    "typeRoots: [
      "./node_modules/@types",
      "./node_modules/@fensak-io"
    ]
    // ...
  }
  // ...
}
```

**with types**
```json5
{
  // ...
  "compilerOptions: {
    // ...
    "types: ["@fensak-io/senc-types"]
    // ...
  }
  // ...
}
```
