---
title: Working offline
description: How the whole environment is published to a git branch and restored on a machine with no network.
---

The full toolchain — the conda environment, the helper binaries and the vendored Cargo
dependencies — is published to an orphan git branch and restored from it on a machine
with no network. That branch is a *transport*, not a history: it is replaced, not
appended to.

## Publishing one

```console
pixi run sandbox-pack      # write the transport into .sandbox-out/
pixi run sandbox-doctor    # verify it without writing anything
pixi run sandbox-publish   # push it to sandbox/<bundle>-<platform>
```

CI does the same thing on every push to `main`, so the branch tracks the lockfiles rather
than whoever last remembered to pack it.

## Restoring one

```console
sh scripts/restore.sh
. .pixi/sandbox-env.sh
pixi install --frozen --offline   # must be a no-op
cargo build --offline             # must use the restored vendor tree
```

`scripts/restore.sh` lives outside pixi on purpose: the point of the airlocked machine is
that pixi need not be installed on it yet. The script reads the branch name off
`.pixi-sandbox.toml`, the same reviewed plan the publisher uses, so the two cannot look
for different branches.

## What the transport carries, and why it is large

`cargo_vendor = true`. With vendoring off, a restored machine would have the toolchain and
no way to obtain the crates in `Cargo.lock` — the environment solved and the build
impossible. Vendored, it can run `cargo build --offline` against the same workspace a
release is built from.

Vendoring is possible because the Cargo manifest is at the repository root, which is the
same constraint that put it there: `cargo install` cannot install from a virtual
manifest, so the directory the conda package is built from has to be a package that is
also the root of its workspace.

## The failure worth recognising

If `cargo` reports

```
error: failed to select a version for the requirement `<crate> = "^1"` (locked to 1.2.2)
candidate versions found which didn't match: 1.2.1
```

the transport is older than `Cargo.lock`: a dependency changed after the branch was last
published. The environment restored correctly — republish the transport, do not edit the
lockfile to match it.
