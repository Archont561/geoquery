/**
 * The one place the project version is read from, and the one place every published
 * surface is checked against it.
 *
 * `[workspace] version` in the root pixi.toml is the single source of truth. It is not
 * Cargo.toml, and that is a deliberate reversal of the usual Rust monorepo: pixi is the
 * manifest that owns the toolchain, the environments and the tasks, the `crates/cli`
 * manifest is a single self-contained package rather than a member of a Cargo workspace,
 * and a `[workspace.package]` table in a Cargo manifest would be a second number that
 * looks authoritative and is checked by nothing. The conda package, the npm client, the
 * PyPI SDK and the Rust crate are then *checked* against pixi's number rather than
 * trusted, because nothing makes four manifests agree on its own.
 *
 * Two callers:
 *   - `pixi run version` / `pixi run version-check`. The latter is in the `gates`
 *     aggregator, so drift fails before a commit lands rather than after a release.
 *   - the docs site, which takes the version from `GEOQUERY_VERSION` — exported by the
 *     docs tasks, so the documented version is decided by the environment that built the
 *     site and never written into it — and falls back to this module when the variable
 *     did not survive. A build started outside pixi still shows the real number; only a
 *     build that can reach neither throws.
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

/**
 * Find the workspace manifest by walking up from `startDir`.
 *
 * Not `new URL("../pixi.toml", import.meta.url)`: the docs site imports this module, and
 * Vite inlines it into `dist/.prerender/chunks/*.mjs`, at which point
 * `import.meta.url` points into the build output and the relative path resolves to a
 * pixi.toml that does not exist. Anchoring on the working directory survives bundling,
 * and works whether the caller is a pixi task (`cwd = "apps/docs"`), a bare
 * `bun run build`, or this script run from the repository root.
 *
 * The `[workspace]` test is what makes "nearest ancestor" safe: a transitive dependency
 * vendored under the repo, or the `crates/cli/pixi.toml` package manifest found by a
 * tool that walks downward, has no workspace table of its own that means what this means,
 * and picking one of those would silently produce the wrong number.
 */
export function workspaceManifestPath(startDir: string = process.cwd()): string {
  let dir = resolve(startDir);
  for (;;) {
    const candidate = join(dir, "pixi.toml");
    if (existsSync(candidate)) {
      const manifest = readFileSync(candidate, "utf8");
      if (/^\[workspace\]$/m.test(manifest)) {
        return candidate;
      }
    }
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`no pixi.toml with a [workspace] table at or above ${startDir}`);
    }
    dir = parent;
  }
}

/**
 * Read a `version = "..."` out of one named TOML table.
 *
 * Scoped to the table rather than the first `version` key in the file, because every
 * manifest here has several: pixi.toml has `[workspace]` and, in a package
 * sub-manifest, `[package]`; a pyproject.toml has `[project]` and `[tool.*]` tables that
 * carry their own (ruff and pytest both do). A line scan rather than a TOML parser on
 * purpose: this is a build-time detail of a docs site and a pre-push check, and one
 * dependency to get right in order to read one field is a bad trade.
 */
function versionInTable(manifestPath: string, table: string, key = "version"): string {
  const manifest = readFileSync(manifestPath, "utf8");
  // `[workspace]` and `[workspace.package]` are different tables, and an anchored match
  // of `^\[workspace\]$` is what keeps the latter from being read as the former.
  const body = manifest.match(
    new RegExp(`^\\[${table.replace(".", "\\.")}\\]$([\\s\\S]*?)^\\[`, "m")
  )?.[1];
  const value = body?.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m"))?.[1];
  if (!value) {
    throw new Error(`no ${key} in [${table}] of ${manifestPath}`);
  }
  return value;
}

/**
 * Read a pixi `[package] version`, following `version = { workspace = true }`.
 *
 * The root pixi.toml is both the workspace and the `geoquery-cli` package, and the package
 * inherits the version rather than restating it — the same rule the Cargo manifests follow,
 * expressed in the only way pixi spells it. Both forms are accepted here so a manifest that
 * goes back to a literal is still read correctly and reported as drift by `check` rather
 * than throwing on the way to being reported.
 */
function pixiPackageVersion(manifestPath: string): string {
  const manifest = readFileSync(manifestPath, "utf8");
  const body = manifest.match(/^\[package\]$([\s\S]*?)^\[/m)?.[1] ?? "";
  if (/^version\s*=\s*\{\s*workspace\s*=\s*true\s*\}\s*$/m.test(body)) {
    return versionInTable(manifestPath, "workspace");
  }
  return versionInTable(manifestPath, "package");
}

/**
 * Read a Cargo `version`, following `version.workspace = true`.
 *
 * A member crate inherits its version from `[workspace.package]` instead of restating it,
 * so that adding a crate is not a ninth place to forget a bump. That inheritance means
 * there is no literal to read in most Cargo manifests, and a checker that insisted on one
 * would be checking a convention the workspace has deliberately abandoned.
 *
 * The inherited value is resolved from the *publishing* workspace's own
 * `[workspace.package]`, not from each member's manifest: a member that hardcodes a
 * version while also claiming `version.workspace = true` is a cargo error, and a member
 * that hardcodes it *instead* is caught by the sweep below, which requires every Cargo
 * manifest under the workspace to inherit rather than restate.
 */
function cargoVersionResolving(manifestPath: string): string {
  const manifest = readFileSync(manifestPath, "utf8");
  const packageBody = manifest.match(/^\[package\]$([\s\S]*?)^\[/m)?.[1] ?? "";
  if (!/^version\.workspace\s*=\s*true\s*$/m.test(packageBody)) {
    // A literal. Readable so the check reports a value rather than throwing, but the
    // `hardcodedCargoVersions` invariant is what actually fails the gate on it.
    return versionInTable(manifestPath, "package");
  }
  return versionInTable(join(cargoWorkspaceRoot(manifestPath), "Cargo.toml"), "workspace.package");
}

/**
 * The Cargo workspace a manifest belongs to: the nearest ancestor directory whose own
 * Cargo.toml declares a `[workspace]` table.
 *
 * Walking up rather than assuming the parent, because the root manifest is its own
 * ancestor: `crates/Cargo.toml` is both the workspace root and a member, so for that one
 * file the workspace root is its own directory, not the directory above it.
 */
function cargoWorkspaceRoot(manifestPath: string): string {
  let dir = dirname(manifestPath);
  for (;;) {
    const candidate = join(dir, "Cargo.toml");
    if (existsSync(candidate) && /^\[workspace\]$/m.test(readFileSync(candidate, "utf8"))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`no [workspace] table above ${manifestPath}`);
    }
    dir = parent;
  }
}

/** Read `[workspace] version` out of the workspace manifest. */
export function workspaceVersion(startDir: string = process.cwd()): string {
  return versionInTable(workspaceManifestPath(startDir), "workspace");
}

/** The repository root, i.e. the directory holding the workspace manifest. */
export function repoRoot(startDir: string = process.cwd()): string {
  return dirname(workspaceManifestPath(startDir));
}

/** `version` out of a `package.json`. */
export function npmVersion(packageJsonPath: string): string {
  const parsed = JSON.parse(readFileSync(packageJsonPath, "utf8")) as { version?: string };
  if (!parsed.version) {
    throw new Error(`no version in ${packageJsonPath}`);
  }
  return parsed.version;
}

/** `version` out of the `[project]` table of a `pyproject.toml`. */
export function pythonVersion(pyprojectPath: string): string {
  return versionInTable(pyprojectPath, "project");
}

/** `version` out of the `[package]` table of a Cargo.toml, following inheritance. */
export function cargoVersion(cargoTomlPath: string): string {
  return cargoVersionResolving(cargoTomlPath);
}

/**
 * Every Cargo manifest in the repository, root first, so the sweep is deterministic.
 *
 * The walk starts at `crates/` rather than the repository root and adds the root manifest
 * explicitly. A whole-root walk is the more general answer and the wrong one: `.pixi/` is a
 * solved environment with manifests in it, and walking it on every version check turns a
 * millisecond task into a directory traversal of a few hundred thousand files.
 */
function cargoManifests(root: string): string[] {
  const found: string[] = [join(root, "Cargo.toml")];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name)
    )) {
      // `target/` is build output and holds no manifests worth checking.
      if (entry.name === "target") continue;
      const full = join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name === "Cargo.toml") found.push(full);
    }
  };
  walk(join(root, "crates"));
  return found;
}

/**
 * Cargo manifests that state a literal version instead of inheriting it.
 *
 * The invariant, not a version comparison: every member under `crates/` must say
 * `version.workspace = true`. A hardcoded `version = "0.1.0"` in one member is a copy
 * that the version check below would still have to notice, and the whole point of
 * `[workspace.package]` is that there is nothing to notice.
 */
export function hardcodedCargoVersions(startDir: string = process.cwd()): string[] {
  const root = repoRoot(startDir);
  return cargoManifests(root).filter((manifest) => {
    // The root is the workspace *and* a package, and it is the one manifest allowed to
    // hold the value every other manifest inherits.
    if (manifest === join(root, "Cargo.toml")) return false;
    const body = readFileSync(manifest, "utf8").match(/^\[package\]$([\s\S]*?)^\[/m)?.[1] ?? "";
    return !/^version\.workspace\s*=\s*true\s*$/m.test(body);
  });
}

/**
 * Every published surface, with paths reported relative to the repository root so a
 * failure names a file rather than an absolute path that differs per machine.
 *
 * The two kinds of manifest appear more than once on purpose. A Cargo manifest is what
 * the binary's `--version` reports, because clap reads `CARGO_PKG_VERSION`; a
 * `[package]` table in a sub-manifest is what `pixi publish` stamps into the artefact's
 * filename and index. A release whose package says 0.1.0 while its binary says 0.1.1
 * installs a binary that disagrees with its own metadata, and neither number is wrong on
 * its own.
 */
export function publishedVersions(startDir: string = process.cwd()): {
  expected: string;
  surfaces: { path: string; version: string }[];
} {
  const root = repoRoot(startDir);
  const sdk = join(root, "python", "geoquery");
  return {
    expected: workspaceVersion(startDir),
    surfaces: [
      { path: "pixi.toml", version: versionInTable(join(root, "pixi.toml"), "workspace") },
      { path: "package.json", version: npmVersion(join(root, "package.json")) },
      {
        path: "packages/client/package.json",
        version: npmVersion(join(root, "packages/client/package.json"))
      },
      {
        path: "python/geoquery/pyproject.toml",
        version: pythonVersion(join(sdk, "pyproject.toml"))
      },
      // The conda packages: what `pixi publish` stamps into a filename and an index.
      {
        path: "python/geoquery/pixi.toml",
        version: versionInTable(join(sdk, "pixi.toml"), "package")
      },
      // The CLI conda package: same manifest as the environment, so its `[package]`
      // version is read out of the root pixi.toml rather than a sub-manifest's.
      {
        path: "pixi.toml [package]",
        version: pixiPackageVersion(join(root, "pixi.toml"))
      },
      // The Cargo manifests: what `geoquery --version` reports, and the library it
      // reports it about. The root `Cargo.toml` is the workspace root as well as the
      // binary's package — which is why `crates/cli/` has no manifest of its own — and it
      // is also where `[workspace.package]` keeps the version every other crate inherits.
      { path: "Cargo.toml", version: cargoVersion(join(root, "Cargo.toml")) },
      ...cargoManifests(root)
        .filter((manifest) => manifest !== join(root, "Cargo.toml"))
        .map((manifest) => ({
          path: relative(repoRoot(startDir), manifest),
          version: cargoVersion(manifest)
        }))
    ]
  };
}

/** `--check`: fail if any published surface disagrees with the workspace version. */
function check(startDir: string = process.cwd()): number {
  // Checked before the versions, because a crate that hardcodes its own version is a
  // structural fault rather than a value mismatch: reporting it as drift would be
  // accurate about the symptom and misleading about the cause.
  const hardcoded = hardcodedCargoVersions(startDir);
  if (hardcoded.length > 0) {
    process.stderr.write("hardcoded Cargo versions (use version.workspace = true):\n");
    for (const manifest of hardcoded) {
      process.stderr.write(`  ${relative(repoRoot(startDir), manifest)}\n`);
    }
    return 1;
  }

  const { expected, surfaces } = publishedVersions(startDir);
  const drifted = surfaces.filter((surface) => surface.version !== expected);
  if (drifted.length === 0) {
    for (const surface of surfaces) {
      process.stdout.write(`${surface.path} ${surface.version}\n`);
    }
    return 0;
  }
  process.stderr.write(`version drift: pixi.toml [workspace] is ${expected}\n`);
  for (const surface of drifted) {
    process.stderr.write(`  ${surface.path} says ${surface.version}\n`);
  }
  process.stderr.write("Bump pixi.toml, or set it to the number the others already carry.\n");
  return 1;
}

if (import.meta.main) {
  const startDir = process.cwd();
  if (process.argv.includes("--check")) {
    process.exit(check(startDir));
  }
  process.stdout.write(workspaceVersion(startDir));
}
