/**
 * Tests for loading the addon, which is the one part of this package that can fail without
 * any TypeScript being wrong.
 *
 * A N-API module cannot be `import`ed — it has to go through `require`, which an ESM package
 * does not have, so `src/native.ts` builds a loader with `createRequire`. If that path is
 * wrong, or the artifact is missing, every assertion in `index.test.ts` fails with a module
 * error that says nothing about the cause. These tests fail with one that does.
 */

import { describe, expect, test } from "bun:test";

import addon from "../src/native.ts";

describe("the addon", () => {
  test("exports exactly one function", () => {
    // The whole argument for this package's shape. A second export is a second operation
    // signature that has to be kept in step with the Rust enum, which is the cost the wire
    // protocol exists to remove — so the count is asserted rather than trusted.
    expect(Object.keys(addon)).toEqual(["invoke"]);
  });

  test("takes and returns JSON text", () => {
    // Not an object on the way in or out. The addon does not get a say in what crosses the
    // boundary, because a binding that decoded the payload here would be a second place
    // where the wire format is written down.
    const returned = addon.invoke('{"transportVersion":1,"operation":"ping","payload":{}}');

    expect(typeof returned).toBe("string");
    expect(JSON.parse(returned)).toMatchObject({ transportVersion: 1, ok: true });
  });

  test("is loaded from a path that resolves", () => {
    // The import above is the assertion: a `createRequire` path that does not resolve
    // throws here rather than inside an unrelated assertion. This test names the reason so
    // that a failure reads as "the addon is missing", not "the protocol is broken".
    expect(addon.invoke).toBeInstanceOf(Function);
  });
});
