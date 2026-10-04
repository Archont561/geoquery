/**
 * Assert the workspace is shaped the way `.knowledge/infrastructure/monorepo.md` says.
 *
 * The monorepo concept makes three structural promises, and all three are the kind that
 * stay true by accident until the day they do not:
 *
 *   1. Adding a crate is one manifest edit. `members = ["crates/*"]` with
 *      `exclude = ["crates/cli"]` means a new directory under `crates/` is a member
 *      without anyone editing a list. A hand-written list would also *look* correct in
 *      review, right up to the crate someone forgot to add — which does not fail, it just
 *      silently stops being linted, tested and published.
 *   2. Lints, versions and release metadata are inherited, never restated. A crate that
 *      hardcodes `edition = "2024"` agrees with the workspace today and pins itself the
 *      day the workspace moves.
 *   3. Tests mirror sources, one file per file. A source file with no test beside it is
 *      meant to be visible; it is only visible if something looks.
 *
 * The first is checked against `cargo metadata` rather than by reading the manifest,
 * because the manifest is the claim and the metadata is what cargo actually resolved —
 * a glob that matches nothing is a correct-looking `members` line and an empty workspace.
 *
 * Run by `pixi run layout-check`, which is in the `gates` aggregator, so drift fails
 * before a commit lands rather than in review.
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

/** Package metadata keys a member must inherit rather than restate. */
const INHERITED_PACKAGE_KEYS = [
  "version",
  "edition",
  "rust-version",
  "license",
  "repository",
  "homepage",
  "authors"
] as const;

/** The one directory under `crates/` that is not a workspace member. */
const EXCLUDED_CRATE = "crates/cli";

/**
 * The one member that restates `[workspace.lints]` instead of inheriting it.
 *
 * `#[napi]` expands to a module constructor carrying its own `#[allow(unsafe_code)]`, and
 * `forbid` is the one level a macro cannot override, so the addon does not compile under
 * the workspace policy. Cargo gives a member no way to override a single inherited lint,
 * so restating the table is the only way to say `deny` here. The manifest says all of
 * this in a comment, and ends it with "must move with it" — which is a promise nothing
 * kept until this check existed. Every key has to match the workspace except this one.
 */
const LINT_EXCEPTION = {
  crate: "crates/node-native",
  allowedDifference: { key: "unsafe_code", workspace: '"forbid"', crate: '"deny"' }
} as const;

export interface Finding {
  /** Repository-relative path of the file the problem is in. */
  readonly where: string;
  /** What is wrong, phrased so the fix is obvious from the message alone. */
  readonly what: string;
}

/** Walk up from `startDir` to the directory holding the workspace `Cargo.toml`. */
export function repoRoot(startDir: string = process.cwd()): string {
  let dir = resolve(startDir);
  for (;;) {
    const candidate = join(dir, "Cargo.toml");
    if (existsSync(candidate) && /^\[workspace\]$/m.test(readFileSync(candidate, "utf8"))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`no Cargo.toml with a [workspace] table at or above ${startDir}`);
    }
    dir = parent;
  }
}

/**
 * Split a TOML document into `[section]` blocks.
 *
 * Deliberately not a TOML parser. The questions here are all of the form "does this key
 * in this table say `workspace = true`", which a parser would answer no better, and a
 * parser is a dependency that has to be vendored for an offline build.
 */
function sections(toml: string): Map<string, string> {
  const found = new Map<string, string>();
  let name = "";
  let body: string[] = [];
  for (const line of toml.split("\n")) {
    const header = /^\s*\[\[?([^\]]+)\]\]?\s*$/.exec(line);
    if (header) {
      if (name) found.set(name, (found.get(name) ?? "") + body.join("\n"));
      name = header[1];
      body = [];
      continue;
    }
    body.push(line);
  }
  if (name) found.set(name, (found.get(name) ?? "") + body.join("\n"));
  return found;
}

/** Every `crates/<name>` directory that holds a `Cargo.toml`. */
export function crateDirs(root: string): string[] {
  return readdirSync(join(root, "crates"), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => `crates/${entry.name}`)
    .filter((dir) => existsSync(join(root, dir, "Cargo.toml")))
    .sort();
}

/** Workspace members as cargo resolved them, repository-relative, excluding the root. */
export function resolvedMembers(root: string): string[] {
  const out = Bun.spawnSync({
    cmd: ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
    cwd: root,
    stdout: "pipe",
    stderr: "pipe"
  });
  if (out.exitCode !== 0) {
    throw new Error(`cargo metadata failed:\n${out.stderr.toString()}`);
  }
  // `--no-deps` already restricts `packages` to workspace members, so the manifest
  // directories are the member list. The root package resolves to "" and is dropped:
  // its manifest is the workspace file, not a crate directory.
  const metadata = JSON.parse(out.stdout.toString()) as {
    packages: { manifest_path: string }[];
  };
  return metadata.packages
    .map((pkg) => relative(root, dirname(pkg.manifest_path)))
    .filter((dir) => dir !== "")
    .sort();
}

/**
 * AC3: adding a crate is one manifest edit, and cargo agrees it worked.
 *
 * The glob is the mechanism, so the check is that the glob's *result* matches the
 * directories on disk — not that the `members` line reads the way it is supposed to.
 */
export function checkMembership(root: string): Finding[] {
  const findings: Finding[] = [];
  const manifest = readFileSync(join(root, "Cargo.toml"), "utf8");
  const workspace = sections(manifest).get("workspace") ?? "";

  const members = /members\s*=\s*\[([^\]]*)\]/.exec(workspace)?.[1] ?? "";
  if (!/^\s*"crates\/\*"\s*,?\s*$/.test(members)) {
    findings.push({
      where: "Cargo.toml",
      what: `[workspace] members must stay the glob ["crates/*"] so adding a crate is one manifest edit, and is [${members.trim()}]`
    });
  }
  const excluded = /exclude\s*=\s*\[([^\]]*)\]/.exec(workspace)?.[1] ?? "";
  if (!/^\s*"crates\/cli"\s*,?\s*$/.test(excluded)) {
    findings.push({
      where: "Cargo.toml",
      what: `[workspace] exclude must be exactly ["${EXCLUDED_CRATE}"], the one directory under crates/ that is not a crate, and is [${excluded.trim()}]`
    });
  }
  if (existsSync(join(root, EXCLUDED_CRATE, "Cargo.toml"))) {
    findings.push({
      where: `${EXCLUDED_CRATE}/Cargo.toml`,
      what: "the cli has no manifest of its own: the root Cargo.toml is its package, so this file must not exist"
    });
  }

  const onDisk = crateDirs(root);
  const resolved = resolvedMembers(root);
  for (const dir of onDisk) {
    if (!resolved.includes(dir)) {
      findings.push({
        where: `${dir}/Cargo.toml`,
        what: "has a manifest but cargo metadata does not list it as a workspace member"
      });
    }
  }
  for (const dir of resolved) {
    if (!onDisk.includes(dir)) {
      findings.push({
        where: "Cargo.toml",
        what: `cargo metadata lists ${dir} as a member, but there is no manifest there`
      });
    }
  }
  return findings;
}

/** Read a lint table into key -> value, ignoring comments and blank lines. */
function lintTable(body: string | undefined): Map<string, string> {
  const table = new Map<string, string>();
  for (const line of (body ?? "").split("\n")) {
    const entry = /^\s*([A-Za-z0-9_:-]+)\s*=\s*(.+?)\s*$/.exec(line);
    if (entry && !line.trimStart().startsWith("#")) table.set(entry[1], entry[2]);
  }
  return table;
}

/**
 * The documented exception, held to the promise its own comment makes.
 *
 * The crate is allowed to restate the workspace lint policy, and allowed to soften
 * exactly one key. Anything else — a lint the workspace added and this crate did not, a
 * level that drifted, a second softened lint nobody announced — is drift wearing the
 * exception as cover, which is worse than no exception at all.
 */
function checkRestatedLints(root: string, path: string, tables: Map<string, string>): Finding[] {
  const findings: Finding[] = [];
  const workspaceTables = sections(readFileSync(join(root, "Cargo.toml"), "utf8"));
  const { key, workspace: fromWorkspace, crate: inCrate } = LINT_EXCEPTION.allowedDifference;

  for (const group of ["rust", "clippy"] as const) {
    const expected = lintTable(workspaceTables.get(`workspace.lints.${group}`));
    const actual = lintTable(tables.get(`lints.${group}`));
    for (const [lint, level] of expected) {
      if (!actual.has(lint)) {
        findings.push({
          where: path,
          what: `[lints.${group}] is missing ${lint}, which [workspace.lints.${group}] sets to ${level}; this crate restates the policy, so it has to restate all of it`
        });
        continue;
      }
      const got = actual.get(lint);
      if (got === level) continue;
      const sanctioned = lint === key && level === fromWorkspace && got === inCrate;
      if (!sanctioned) {
        findings.push({
          where: path,
          what: `[lints.${group}] ${lint} is ${got} but the workspace says ${level}; the only sanctioned difference is ${key} = ${inCrate}`
        });
      }
    }
    for (const lint of actual.keys()) {
      if (!expected.has(lint)) {
        findings.push({
          where: path,
          what: `[lints.${group}] sets ${lint}, which [workspace.lints.${group}] does not; add it to the workspace or drop it here`
        });
      }
    }
  }
  return findings;
}

/** AC2: every member inherits its metadata, lints and dependency versions. */
export function checkInheritance(root: string, members: string[]): Finding[] {
  const findings: Finding[] = [];
  for (const dir of members) {
    const path = join(dir, "Cargo.toml");
    const tables = sections(readFileSync(join(root, path), "utf8"));

    const pkg = tables.get("package") ?? "";
    for (const key of INHERITED_PACKAGE_KEYS) {
      const inherited = new RegExp(
        `^\\s*${key}\\s*(\\.workspace\\s*=\\s*true|=\\s*\\{[^}]*workspace\\s*=\\s*true)`,
        "m"
      );
      const restated = new RegExp(`^\\s*${key}\\s*=\\s*(?!\\{)`, "m");
      if (restated.test(pkg) && !inherited.test(pkg)) {
        findings.push({
          where: path,
          what: `[package] ${key} is written out; inherit it with ${key}.workspace = true`
        });
      }
    }

    if (dir === LINT_EXCEPTION.crate) {
      findings.push(...checkRestatedLints(root, path, tables));
    } else {
      const lints = tables.get("lints");
      if (lints === undefined || !/workspace\s*=\s*true/.test(lints)) {
        findings.push({
          where: path,
          what: "has no [lints] workspace = true, so the workspace lint policy does not reach it"
        });
      }
    }

    for (const [name, body] of tables) {
      if (!/(^|-)dependencies$/.test(name)) continue;
      for (const line of body.split("\n")) {
        if (/^\s*(#|$)/.test(line)) continue;
        const entry = /^\s*([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(line);
        if (!entry) continue;
        const [, crate, value] = entry;
        if (/workspace\s*=\s*true/.test(value)) continue;
        findings.push({
          where: path,
          what: `[${name}] ${crate} pins its own version; declare it in [workspace.dependencies] and say { workspace = true }`
        });
      }
    }
  }
  return findings;
}

/**
 * AC4: one test file per source file, for every crate.
 *
 * `crates/cli` is included even though its manifest is the repository root: its sources
 * and tests live beside each other like everyone else's, which is the whole point of the
 * `[[bin]] path` and `[[test]] path` lines that put them there.
 */
export function checkMirror(root: string, crates: string[]): Finding[] {
  const findings: Finding[] = [];
  const rustFiles = (dir: string): string[] =>
    existsSync(dir)
      ? readdirSync(dir)
          .filter((f) => f.endsWith(".rs"))
          .sort()
      : [];

  for (const dir of crates) {
    const src = rustFiles(join(root, dir, "src"));
    const tests = rustFiles(join(root, dir, "tests"));
    if (src.length === 0) continue;
    if (tests.length === 0) {
      findings.push({ where: dir, what: "has sources but no tests/ directory beside them" });
      continue;
    }
    for (const file of src) {
      if (!tests.includes(file)) {
        findings.push({ where: `${dir}/src/${file}`, what: `has no tests/${file} beside it` });
      }
    }
    for (const file of tests) {
      if (!src.includes(file)) {
        findings.push({
          where: `${dir}/tests/${file}`,
          what: `tests a source file that does not exist; the mirror is one test file per source file`
        });
      }
    }
  }
  return findings;
}

/** Every crate directory with sources, including the cli whose manifest is the root. */
export function crateSourceDirs(root: string): string[] {
  return readdirSync(join(root, "crates"), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => `crates/${entry.name}`)
    .filter((dir) => existsSync(join(root, dir, "src")))
    .sort();
}

export function check(startDir: string = process.cwd()): number {
  const root = repoRoot(startDir);
  const findings = [
    ...checkMembership(root),
    ...checkInheritance(root, crateDirs(root)),
    ...checkMirror(root, crateSourceDirs(root))
  ];
  if (findings.length === 0) {
    const crates = crateSourceDirs(root);
    process.stdout.write(
      `workspace layout matches the specification: ${crates.length} crates, ` +
        `members from the crates/* glob, ${EXCLUDED_CRATE} excluded, tests mirroring sources\n`
    );
    return 0;
  }
  process.stderr.write("workspace layout does not match the specification:\n");
  for (const finding of findings) {
    process.stderr.write(`  ${finding.where}: ${finding.what}\n`);
  }
  process.stderr.write(
    "\nEither fix the workspace, or change .knowledge/infrastructure/monorepo.md and this\n" +
      "check together — the point is that the two cannot drift apart quietly.\n"
  );
  return 1;
}

if (import.meta.main) {
  process.exit(check(process.cwd()));
}
