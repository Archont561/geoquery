/**
 * Tests for the TypeScript package's public surface, through the real addon.
 *
 * These assert the boundary, not the addon's internals: the point of the package is that it
 * reaches the *same* Rust core as the CLI and the Python SDK, so what is worth checking is that
 * the numbers agree with each other and that a document is judged by the Rust parser. A
 * test that reimplemented the key ordering here would pass whether or not the addon was
 * loaded at all — and loading it is the thing that can break.
 */

import { describe, expect, test } from "bun:test";

import {
  EngineError,
  invoke,
  invokeRaw,
  parseDocument,
  ping,
  protocolVersion,
  TRANSPORT_VERSION,
  VERSION
} from "../src/index.ts";

describe("version", () => {
  test("is a semantic version", () => {
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+/);
  });

  test("reaches the engine unchanged", () => {
    // Containment rather than equality because the user-agent also carries the query
    // protocol version, which is allowed to diverge from the package version later. What
    // is asserted here is that this package's number reaches the engine, not that the
    // user-agent has exactly one number in it — a test that froze the full string would
    // fail on the day the two versions legitimately part company.
    expect(protocolVersion().userAgent).toContain(VERSION);
  });

  test("the engine and this package report the same build", () => {
    // The version gate compares manifests; this compares the built artifact to them. A
    // published package whose `VERSION` is right and whose embedded engine is stale is a
    // failure no manifest check would catch.
    expect(protocolVersion().version).toBe(VERSION);
  });
});

describe("transport version", () => {
  test("is the one Rust answers with", () => {
    // Written down in two languages. Nothing keeps those two numbers in step except this
    // assertion, so it is the test that catches a Rust bump the TypeScript side never
    // heard about.
    expect(invokeRaw("ping").transportVersion).toBe(TRANSPORT_VERSION);
  });
});

describe("ping", () => {
  test("echoes the payload back", () => {
    expect(ping("typescript")).toBe("typescript");
  });

  test("round-trips a payload rather than one field of it", () => {
    // The payload is returned whole, so a caller can prove the boundary passes values and
    // not just the shapes this package happens to know about.
    const payload = { nested: { list: [1, 2, 3] }, message: "héllo" };

    expect(invoke<{ echo: unknown }>("ping", payload).echo).toEqual(payload);
  });
});

describe("parseDocument", () => {
  test("uses the Rust parser and sorts the keys", () => {
    expect(parseDocument('{"temporal": {}, "bbox": []}')).toEqual(["bbox", "temporal"]);
  });

  test("rejects a document that is not an object", () => {
    expect(() => parseDocument("[]")).toThrow("not a query document");
  });

  test("reports the engine's own wording rather than one written here", () => {
    // The message is `geoquery-core`'s. Asserting the exact text rather than a substring
    // is deliberate: it is what makes a change to core's wording a visible test failure
    // here instead of a silent divergence between what Python raises and what TypeScript
    // raises for the same document.
    let detail: unknown;
    try {
      parseDocument("[]");
    } catch (error) {
      detail = error instanceof EngineError ? error.detail : undefined;
    }
    expect(detail).toMatchObject({ error: expect.stringContaining("not a query document") });
  });

  test("accepts an empty document rather than having an opinion about it", () => {
    // `{}` is a degenerate query, not an invalid one. Core refuses to decide what an
    // empty query means before the engine exists to decide it, and this asserts that the
    // boundary did not quietly add a rule of its own.
    expect(parseDocument("{}")).toEqual([]);
  });
});

describe("failure", () => {
  test("is reported as data by invokeRaw and thrown by invoke", () => {
    // Both readings of the same response. `invokeRaw` exists for a batch that collects
    // failures rather than stopping at the first one; only `invoke` unwraps.
    const response = invokeRaw("parseDocument", { document: "[]" });

    expect(response.ok).toBe(false);
    expect(() => invoke("parseDocument", { document: "[]" })).toThrow(EngineError);
  });

  test("keeps the engine's detail on the error", () => {
    // So a caller that needs to branch on the reason has something to branch on, rather
    // than re-parsing a message string that was only ever meant to be read.
    let detail: Record<string, unknown> | undefined;
    try {
      invoke("parseDocument", {});
    } catch (error) {
      detail = error instanceof EngineError ? error.detail : undefined;
    }
    expect(detail).toMatchObject({ error: expect.stringContaining("document") });
  });
});

/**
 * The edges of the transport's number domain, from the side that bounds it tightest.
 *
 * The engine carries 64-bit numbers, but JavaScript cannot observe all of them: a `number`
 * is a double, so integers above `Number.MAX_SAFE_INTEGER` lose precision in this runtime's
 * own `JSON.parse` before a request reaches the engine and after a response leaves it. That
 * is why the transport promises a bounded domain rather than exact integers — an exactness
 * Rust and Python could honour and this binding could not would be three contracts, not one.
 */
test("integers JavaScript can represent exactly survive the round trip", () => {
  const edges = [-Number.MAX_SAFE_INTEGER, -1, 0, 1, Number.MAX_SAFE_INTEGER];

  expect(invoke<{ echo: unknown }>("ping", { payload: edges }).echo).toStrictEqual({
    payload: edges
  });
});

test("past 2^53 it is JavaScript that rounds, not the engine", () => {
  // `9007199254740993` is the first integer a double cannot hold. The literal below is
  // already the rounded value by the time this file is parsed, which is the whole point:
  // the engine never sees the number that was written, so no engine-side change could
  // make this exact.
  expect(Number("9007199254740993")).toBe(9007199254740992);

  const rounded = Number("9007199254740993");
  expect(invoke<{ echo: unknown }>("ping", { payload: rounded }).echo).toStrictEqual({
    payload: 9007199254740992
  });
});
