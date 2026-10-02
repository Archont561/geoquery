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
 * A JSON value built only from things JSON represents exactly: strings, booleans, null,
 * integers, objects and arrays.
 *
 * `undefined` is deliberately absent, and that is the generator's one non-obvious omission:
 * `JSON.stringify` drops an `undefined` array element and turns an `undefined` object value
 * into `null`, so a payload containing one is not the payload that was sent. A property over
 * undefined would be testing the serialiser's coercion table, which is not the transport's
 * contract.
 *
 * Floats are excluded, and not because they are awkward. They are excluded because
 * `serde_json`'s parser is not correctly rounded: for `4.188295992926135e-215` it produces
 * a `f64` one unit in the last place away from the one Rust's own `str::parse::<f64>()`
 * produces, and from the one JavaScript and Python produce. The wire then reports a payload
 * that came in as one double and left as another.
 *
 * That is a property of the JSON library rather than of this transport, it is fixed by
 * serde_json's `arbitrary_precision` feature rather than by anything here, and it is
 * pinned as a canary below so that a serde_json upgrade which fixes it fails a test that
 * asks to be updated — at which point this generator can take floats back.
 */
const exactJson = fc.letrec((tie) => ({
  value: fc.oneof(
    { depthSize: "small" },
    fc.string(),
    fc.integer(),
    fc.boolean(),
    fc.constant(null),
    tie("array"),
    tie("object")
  ),
  array: fc.array(tie("value"), { maxLength: 4 }),
  object: fc.dictionary(fc.string(), tie("value"), { maxKeys: 4 })
})).value;

test("ping round-trips every payload JSON represents exactly", () => {
  fc.assert(
    fc.property(exactJson, (payload) => {
      const echo = invoke<{ echo: unknown }>("ping", { payload }).echo;

      // Whole payload rather than one field: `ping` exists to prove the boundary passes
      // values and not just the shapes this client knows about, so asserting a single key
      // would pass against an adapter that dropped everything else.
      expect(echo).toEqual({ payload });
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

test("a response always carries the transport version this client speaks", () => {
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

test("canary: an f64 literal at an extreme exponent does not survive the round trip", () => {
  // This test is expected to fail when serde_json's parser becomes correctly rounded, and
  // that is the point of writing it. A test asserting a known bug is a claim about the
  // world with a date on it; when the world changes, this fails and asks whether
  // `exactJson` can take floats back yet.
  //
  // If it starts passing without anyone updating the comment above, the property test has
  // been silently weaker than it reads for a release.
  const payload = { value: 4.188295992926135e-215 };

  expect(invoke<{ echo: unknown }>("ping", { payload }).echo).not.toEqual({ payload });
});
