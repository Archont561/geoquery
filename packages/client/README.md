# @archont561/geoquery-client

The TypeScript client for [geoquery](https://github.com/Archont561/geoquery).

Geoquery is currently in Phase 0: this package exposes the versioned client foundation while
the single-source query engine is being built. See the repository README for the supported
surface and release status.

## Install from npm

```sh
bun add @archont561/geoquery-client
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
bun add @archont561/geoquery-client
```

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))
