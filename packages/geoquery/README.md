# @archont561/geoquery

TypeScript bindings for [Geoquery](https://github.com/Archont561/geoquery), backed by the
same Rust core as the CLI and Python SDK.

This package is not an HTTP client. It loads the `geoquery-node-native` N-API addon and
speaks to the Rust engine in-process over Geoquery's versioned JSON transport. Geoquery is
currently in Phase 0, so the package exposes the version, transport, ping, and query-document
parsing surface while the single-source query engine is being built.

## Install from npm

```sh
bun add @archont561/geoquery
```

## Install from GitHub Packages

Configure the `@archont561` scope in your project's `.npmrc` with a GitHub token that has
`read:packages` permission:

```ini
@archont561:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

Then install the same package name:

```sh
bun add @archont561/geoquery
```

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))
