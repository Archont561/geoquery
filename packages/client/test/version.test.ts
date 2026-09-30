import { describe, expect, test } from "bun:test";

import { USER_AGENT, VERSION } from "../src/index.ts";

/**
 * The client's version is read from package.json at import time, which is the same file
 * `pixi run version-check` compares against the rest of the project. These tests do not
 * re-implement that check: they assert the wiring — that the number survives the import
 * and that the user-agent is derived from it — and the cross-manifest agreement is
 * enforced once, in one place, by a task that is in the `gates` aggregator.
 */
describe("version", () => {
  test("is a semantic version", () => {
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+/);
  });

  test("reaches the user-agent unchanged", () => {
    expect(USER_AGENT).toBe(`geoquery-client/${VERSION}`);
  });
});
