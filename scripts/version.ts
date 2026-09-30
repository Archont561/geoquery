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
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

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

/** `version` out of the `[package]` table of a Cargo.toml. */
export function cargoVersion(cargoTomlPath: string): string {
  return versionInTable(cargoTomlPath, "package");
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
  const crates = join(root, "crates");
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
      { path: "crates/pixi.toml", version: versionInTable(join(crates, "pixi.toml"), "package") },
      // The Cargo manifests: what `geoquery --version` reports, and the library it
      // reports it about. `crates/Cargo.toml` is the workspace root as well as a
      // package, which is why the binary has no manifest of its own.
      { path: "crates/Cargo.toml", version: cargoVersion(join(crates, "Cargo.toml")) },
      {
        path: "crates/core/Cargo.toml",
        version: cargoVersion(join(crates, "core", "Cargo.toml"))
      }
    ]
  };
}

/** `--check`: fail if any published surface disagrees with the workspace version. */
function check(startDir: string = process.cwd()): number {
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
