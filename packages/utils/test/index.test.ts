/**
 * The package's public surface, asserted rather than assumed.
 *
 * `packages/core/tests/lib.rs` does the same job in Rust — "the crate root is the whole
 * public surface" — and the reason applies here: a fixture helper that leaks into a
 * published package's dependency graph is a test-only dependency shipped to users, and the
 * way that happens is an export nobody re-reads.
 */

import { describe, expect, test } from "bun:test";
import * as fixtures from "../src/fixtures.ts";
import * as utils from "../src/index.ts";

describe("the package root", () => {
  test("re-exports everything from the fixtures module", () => {
    // Two entry points (`@geoquery/utils` and `@geoquery/utils/fixtures`) over one
    // implementation. If the root stops re-exporting a name, a consumer that moved to the
    // root would get `undefined` at runtime while still type-checking — because the
    // subpath still exports it.
    expect(Object.keys(utils).sort()).toEqual(Object.keys(fixtures).sort());
  });

  test("exports only fixture infrastructure", () => {
    expect(Object.keys(utils).sort()).toEqual(["createFixture"]);
  });

  test("is private, so nothing published can depend on it", async () => {
    // The property the package's `private: true` exists to guarantee. Asserted from the
    // manifest because a consumer's `dependencies` entry is the thing that would ship.
    const manifest = (await import("../package.json", { with: { type: "json" } })).default;

    expect(manifest.private).toBe(true);
  });
});
