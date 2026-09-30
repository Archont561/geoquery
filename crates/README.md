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
| `../Cargo.toml` | the workspace root, and the `geoquery-cli` package `pixi publish` builds |
| `../pixi.toml` | the environment, and the same package's conda recipe |
| `../deny.toml` | the dependency policy `pixi run lint` enforces (via `cargo deny`) |
| `package.json` | the turbo façade for the Rust workspace: `build`/`test`/`lint`/`typecheck`/`cov`/`fmt` run cargo |
| `core/` | `geoquery-core`: the query language — documents, versions, rules |
| `cli/src/main.rs` | the binary: argument parsing, messages, exit codes |
| `cli/tests/main.rs` | the binary's tests: it is run, and its stdout, stderr and exit code are the assertions |
| `types/`, `adapter-*/`, `http/`, `mcp/`, `tui/`, `xtask/` | the remaining crates from the roadmap, declared with the dependencies each will need |

Each crate keeps its tests in `tests/`, one file per file in `src/`: `core/src/document.rs`
is tested by `core/tests/document.rs`. `cli/tests/main.rs` is registered by `[[test]]` in
`../Cargo.toml` because the binary's manifest is not in the binary's directory; every other
crate gets the layout from cargo for free.

The workspace manifests are at the repository root rather than here, and there is no
`cli/Cargo.toml`. Both follow from the comment at the top of `../Cargo.toml`: the directory
a conda package is built from must be the root of a workspace that is itself a package,
because `cargo install` cannot install from a virtual manifest. Putting the manifests at the
root is also what lets the offline sandbox vendor the whole dependency graph.

Installed from a channel with `pixi global install geoquery-cli`, or from a checkout with
`pixi run publish-dist` followed by the artefact in `dist/`.
