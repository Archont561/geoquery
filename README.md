# Offline sandbox (orphan branch)

Built 2026-10-03T21:54:21Z from commit `6d4db33` for platform `linux-64`.
`pixi.lock` sha256 `cc9e88c9cff77277b45600c1c936e334cc3dc9f165b66f78ae38d782dbf81451`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 471.5 MiB | 1933.7 MiB | 83 |

Cargo dependencies: **363 crates**, 376.5 MiB (loose) from `Cargo.lock` sha256 `cc77f8b042de…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.98.1 (797e8a9bc 2026-08-05); rustc 1.98.1 (48a229cea 2026-09-01).

## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
