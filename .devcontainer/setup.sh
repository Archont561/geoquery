#!/usr/bin/env bash
# Devcontainer post-create setup. Keep the environment definition in pixi.toml
# and pixi.lock; this script only installs the small set of host tools needed to
# materialise and use that environment.
set -euo pipefail

pixi --version

# Baseline CLI tooling that the pixi image does not ship with.
pixi global install git gh

# A C toolchain is needed to build the Rust crates. The conda compiler binaries
# are prefixed with the target triple, so expose the unprefixed names that build
# scripts expect.
pixi global install \
  --expose cc \
  --expose gcc \
  --expose ar=x86_64-conda-linux-gnu-ar \
  c-compiler

# Materialise the project environment from the committed lockfile.
pixi install --locked --all

# Install the Bun workspace, editable Python SDK, and git hooks through the
# repository's own task graph.
pixi run setup

# Install the optional agent CLI and refresh its model catalogue, matching the
# development environment used by pixi-sandbox. This is container tooling rather
# than a project dependency: the CLI is how the repository is worked on, not an
# input its build needs, so it does not belong in the task graph every clone
# resolves. Bun lives in the project environment, so reach it through `pixi run`
# instead of assuming it is on this script's PATH.
#
# The install comes first because `bun pm bin -g` reads a store that the first
# global install initialises. Asked before that, it exits non-zero having printed
# nothing, and under `set -e` the assignment below takes this script down with it:
# a first setup reported "No package.json was found ... Run "bun init"" and left no
# agent CLI at all, which reads like a broken image rather than a wrong order.
pixi run -e default bun add -g opencode-ai@latest
BUN_BIN="$(pixi run -e default bash -c 'bun pm bin -g')"

# A dangling link in /usr/local/bin is worse than no link: `opencode` then fails
# with "No such file or directory", which is indistinguishable from the CLI never
# having been installed. Name the path that was checked instead.
if [[ ! -x "$BUN_BIN/opencode" ]]; then
  printf 'setup: no opencode launcher at %s/opencode after bun add -g\n' "$BUN_BIN" >&2
  exit 1
fi
ln -sfn "$BUN_BIN/opencode" /usr/local/bin/opencode
"$BUN_BIN/opencode" models --refresh
