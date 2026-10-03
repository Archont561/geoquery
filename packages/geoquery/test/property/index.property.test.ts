/**
 * Property tests for the transport, through the real addon.
 *
 * The example-based tests in `../index.test.ts` pin the cases a reader can see in the
 * engine's source. These do the opposite: they generate the inputs, so the property being
 * asserted — that whatever goes in comes back — is checked against values nobody chose,
 * including the ones nobody would have thought to write down.
 *
 * The separator exists because the two kinds fail differently. An example that fails names
 * the behaviour it was documenting; a property that fails hands back a counterexample and
 * shrinks it to something readable, but only if it is not sharing a file with assertions
 * whose failures have nothing to do with it.
 */

import { expect, test } from "bun:test";
import fc from "fast-check";

import { invoke, invokeRaw, parseDocument } from "../../src/index.ts";

/**
 * Any JSON value, built the way the Rust side builds its generator.
 *
 * Floats are in here because getting them to be here is the point. `serde_json`'s default
 * float handling is not round-trip exact in either direction: its serialiser wrote
 * `432288997.760232` for a double whose exact value is `432288997.76023203`, and its parser
 * rounded `4.188295992926135e-215` a unit in the last place away from the one Rust,
 * JavaScript and Python all agree it is. `ping` promises the payload comes back unchanged,
 * and in this project the payload is coordinates, so that promise has to be exact rather
 * than nearly exact. `Cargo.toml` turns on serde_json's `float_roundtrip` feature to deliver
 * it; these tests are what would notice it being turned off.
 *
 * `undefined` is deliberately absent: `JSON.stringify` drops an `undefined` array element and
 * turns an `undefined` object value into `null`, so a payload containing one is not the
 * payload that was sent. `-0` is absent for the same reason: JSON text has no signed-zero
 * spelling, so it serialises as `0`. Properties over either would be testing the
 * serialiser's coercion table, which is not the transport's contract.
 */
const jsonNumber = fc
  .double({ noDefaultInfinity: true, noNaN: true })
  .filter((value) => !Object.is(value, -0));

const anyJson = fc.letrec((tie) => ({
  value: fc.oneof(
    { depthSize: "small" },
    fc.string(),
    fc.integer(),
    jsonNumber,
    fc.boolean(),
    fc.constant(null),
    tie("array"),
    tie("object")
  ),
  array: fc.array(tie("value"), { maxLength: 4 }),
  object: fc.dictionary(fc.string(), tie("value"), { maxKeys: 4 })
})).value;

test("ping round-trips any payload, floats included", () => {
  fc.assert(
    fc.property(anyJson, (payload) => {
      const echo = invoke<{ echo: unknown }>("ping", { payload }).echo;

      // Whole payload rather than one field: `ping` exists to prove the boundary passes
      // values and not just the shapes this package knows about, so asserting a single key
      // would pass against an adapter that dropped everything else.
      //
      // `toStrictEqual` rather than `toEqual` so arrays, objects, and numeric edge cases
      // are compared without coercion. Values that JSON cannot spell, such as `-0`, are
      // filtered out of the arbitrary above before they reach the transport.
      expect(echo).toStrictEqual({ payload });
    })
  );
});

test("ping round-trips strings the example tests would not have tried", () => {
  fc.assert(
    fc.property(fc.string(), (message) => {
      expect(invoke<{ echo: { message: string } }>("ping", { message }).echo.message).toBe(message);
    })
  );
});

test("a response always carries the transport version this package speaks", () => {
  fc.assert(
    fc.property(fc.string(), (document) => {
      // `parseDocument` because it is the operation that most often fails, so this covers
      // both outcomes: a version is reported whether the operation succeeded or not.
      const response = invokeRaw("parseDocument", { document });

      expect(response.transportVersion).toBe(1);
    })
  );
});

test("a document's keys come back sorted, whatever order they were written in", () => {
  fc.assert(
    fc.property(
      fc.uniqueArray(fc.string({ minLength: 1 }), { minLength: 0, maxLength: 12 }),
      (keys) => {
        const document = JSON.stringify(Object.fromEntries(keys.map((key) => [key, null])));

        expect(parseDocument(document)).toEqual([...keys].sort());
      }
    )
  );
});

test("a document with one key reports that key", () => {
  fc.assert(
    fc.property(fc.string({ minLength: 1 }), (key) => {
      expect(parseDocument(JSON.stringify({ [key]: null }))).toEqual([key]);
    })
  );
});

test("the two floats that exposed serde_json's default handling still round-trip", () => {
  // Named rather than generated, because generated floats prove the general case and these
  // prove the specific defects are still fixed. This started life as a canary asserting the
  // round trip was *not* exact; it became this when `float_roundtrip` made it exact, and it
  // would start failing again if the feature were ever dropped from `Cargo.toml`.
  for (const value of [4.188295992926135e-215, 432288997.76023203]) {
    // `toBe` is `Object.is`, so this is a bit-for-bit comparison rather than an equality
    // that would wave through a value one unit in the last place away.
    expect(invoke<{ echo: { value: number } }>("ping", { value }).echo.value).toBe(value);
  }
});
