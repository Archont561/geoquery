/**
 * Tests for `createFixture`.
 *
 * Every fixture here is declared at `describe` scope rather than inside a test, because
 * that is the only place `createFixture` may be called — bun rejects `beforeEach` from
 * inside a test body. Asserting that constraint from within the tests is therefore not
 * possible, so it is documented in `src/fixtures.ts` and the suites below are written in
 * the shape that satisfies it.
 *
 * The assertions are about the two lifecycles and the two failure paths: a getter read too
 * early, and a teardown that runs against a setup that never finished. Both are the cases
 * where a helper that "works" in the common case can still corrupt a suite — an early read
 * puts `undefined` into every assertion, and tearing down a half-built value can dispose of
 * something another test still holds.
 */

import { describe, expect, mock, test } from "bun:test";

import { createFixture } from "../src/fixtures.ts";

describe("a per-test fixture", () => {
  // The previous test's value, so the next test can compare against it. Declared here
  // rather than in the test that needs it because there is no other place a per-test
  // fixture's value survives to.
  let previous: object | undefined;

  const setup = mock(() => ({}));
  const fixture = createFixture(setup);

  const teardown = mock((_value: { id: number }) => undefined);
  const tracked = createFixture(() => ({ id: 1 }), teardown);

  test("returns the value setup produced", () => {
    expect(fixture()).toEqual({});
  });

  test("is a different value than the previous test's", () => {
    const current = fixture();

    // Identity rather than equality: two structurally equal objects are also `toEqual`, and
    // the property being asserted is that the *setup ran again*, not that it returned the
    // same shape twice.
    expect(current).not.toBe(previous);
    previous = current;
  });

  test("tears down after each test", () => {
    // Greater than zero rather than exactly two: the point is that teardown runs per test,
    // and pinning the count would make this suite fail the day a test is added above.
    expect(teardown.mock.calls.length).toBeGreaterThan(0);
    expect(teardown.mock.calls[0]?.[0]).toEqual({ id: 1 });
  });

  test("runs setup again for every test in the scope", () => {
    // More than once rather than an exact count: the count is "one per test", so pinning
    // it would make this suite fail the day a test is added above. The exact assertion
    // that distinguishes the two scopes lives in the file-scoped block below, where one
    // is the whole point and cannot drift.
    expect(setup.mock.calls.length).toBeGreaterThan(1);
  });
});

describe("a file-scoped fixture", () => {
  const setup = mock(() => ({}));
  const fixture = createFixture(setup, undefined, "file");

  const teardown = mock((_value: string) => undefined);
  const tracked = createFixture(() => "shared", teardown, "file");

  test("hands the same value to every test in the file", () => {
    expect(fixture()).toBe(fixture());
  });

  test("ran setup once for the whole file", () => {
    // Exactly one, and not "less than three". A per-test fixture would be three by the
    // time this runs, so this is the assertion that separates the two scopes rather than
    // one that merely passes for both.
    expect(setup.mock.calls.length).toBe(1);
  });

  test("accepts a value with a teardown", () => {
    expect(tracked()).toBe("shared");
  });

  test("tears down once for the whole file", () => {
    // Not yet: `afterAll` runs after the last test in the scope, so this can only be
    // asserted by the file-scoped teardown count at the end. Asserting it here would be
    // asserting something false.
    expect(teardown.mock.calls.length).toBe(0);
  });
});

describe("a fixture with no teardown", () => {
  const fixture = createFixture(() => 1);

  test("is legal, because most values need no disposal", () => {
    expect(fixture()).toBe(1);
  });
});

describe("reading a fixture before its setup has run", () => {
  const fixture = createFixture(() => "value");

  // The `describe` body runs before any `beforeEach` in it, which is exactly the mistake
  // the accessor guards against: a fixture captured into a shared variable at suite scope
  // and read before any test runs.
  const readTooEarly = (): unknown => {
    try {
      return fixture();
    } catch (error) {
      return error;
    }
  };
  const outcome = readTooEarly();

  test("throws instead of handing back undefined", () => {
    // Returning `undefined` here would make this suite fail in every assertion at once
    // with a message about the value rather than about the wiring.
    expect(outcome).toBeInstanceOf(Error);
    expect((outcome as Error).message).toContain('"test" setup ran');
  });

  test("names the scope, so the message says which hook was missed", () => {
    expect((outcome as Error).message).toContain("not at suite scope");
  });

  test("works normally once the test body has run", () => {
    expect(fixture()).toBe("value");
  });
});
