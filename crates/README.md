# geoquery-cli

The `geoquery` command-line client.

```console
$ geoquery --version
$ geoquery check query.json
query.json is a query object with 2 keys: execution, limit
```

`geoquery check` reads a query document and reports its shape without contacting
anything. `geoquery query --service <URL> --query <file>` refuses, with exit code 3,
until the query engine lands — see the repository README for the status.

## Layout

| Path | What it is |
| --- | --- |
| `Cargo.toml` | the workspace root, and the package this directory is published as |
| `core/` | `geoquery-core`: the query language — documents, versions, rules |
| `cli/src/main.rs` | the binary: argument parsing, messages, exit codes |
| `pixi.toml` | the conda package built by `pixi publish` |
| `deny.toml` | the dependency policy `pixi run deny` enforces |

There is no `cli/Cargo.toml`. See the comment at the top of `Cargo.toml`: the directory a
conda package is built from must be the root of a workspace that is itself a package,
because `cargo install` cannot install from a virtual manifest.

Installed from a channel with `pixi global install geoquery-cli`, or from a checkout with
`pixi run publish-dist` followed by the artefact in `dist/`.
