/**
 * Test fixtures for `bun test`, the TypeScript half of `rstest`'s `#[fixture]`.
 *
 * A fixture is a value a suite needs set up before a test and torn down after it: a
 * built addon, a temporary directory, a scripted transport. Writing that wiring by hand
 * every time is how suites end up with three subtly different one-off `beforeEach` blocks,
 * so the two lifecycles a fixture actually has are expressed once here:
 *
 * - `"test"` — a fresh value per test (`beforeEach`/`afterEach`). The default, because
 *   isolation is the safe choice: nothing a test does to its fixture can leak into the
 *   next one.
 * - `"file"` — one value for the whole file (`beforeAll`/`afterAll`). For expensive,
 *   immutable setup — a compiled addon, a module — where per-test freshness buys nothing
 *   and per-test cost is real.
 *
 * The scope default is `"test"` rather than `"file"` because the expensive case is also the
 * rarer one, and a fixture that is fresh by default cannot surprise a later reader by
 * having leaked state between tests. Opting *into* sharing is a visible decision; opting
 * out of isolation is not.
 *
 * Call `createFixture` at suite scope — a `describe` body or module top level, where bun
 * accepts `beforeAll` and friends. It registers its hooks there and returns a getter that
 * only works inside the wired lifecycle:
 *
 * ```ts
 * const addon = createFixture(loadAddon, undefined, "file");
 *
 * test("speaks the transport version", () => {
 *   expect(addon().version).toBe(1);
 * });
 * ```
 *
 * The getter throws rather than returning `undefined` when it is called before setup has
 * run, so a fixture wired into the wrong scope fails at the call site with a message
 * naming the scope, instead of failing every assertion with `undefined`.
 */

import { afterAll, afterEach, beforeAll, beforeEach } from "bun:test";

/** Create the fixture's value. May be async. */
export type Setup<T> = () => T | Promise<T>;

/** Dispose of the fixture's value. Receives what `Setup` produced. */
export type Teardown<T> = (value: T) => void | Promise<void>;

/** How long a fixture lives: one test, or the whole file. */
export type FixtureScope = "file" | "test";

/** The accessor `createFixture` returns. Call it inside a test. */
export type Fixture<T> = () => T;

/**
 * Register a fixture and return its accessor.
 *
 * @param setup    runs before the scope's tests, once per scope
 * @param teardown runs after the scope's tests, once per scope; skipped when setup did
 *                 not finish, so a half-built fixture is never torn down as if it were whole
 * @param scope    `"test"` for a fresh value per test (the default), `"file"` for one
 *                 value shared by the whole file
 */
export function createFixture<T>(
  setup: Setup<T>,
  teardown: Teardown<T> | undefined = undefined,
  scope: FixtureScope = "test"
): Fixture<T> {
  // The holder's presence *is* the state machine: assigned once setup finished building,
  // cleared before teardown disposes. "Tear down only what setup built" is then a fact
  // about the type of this variable rather than a runtime flag that can disagree with it.
  let holder: { readonly value: T } | undefined;

  const failEarly = (): never => {
    throw new Error(
      `fixture was read before its "${scope}" setup ran — read it inside a test, not at suite scope`
    );
  };

  const build = async (): Promise<void> => {
    holder = { value: await setup() };
  };
  const dispose = async (): Promise<void> => {
    const built = holder;
    holder = undefined;
    if (built === undefined) {
      return;
    }
    await teardown?.(built.value);
  };

  if (scope === "file") {
    beforeAll(build);
    afterAll(dispose);
  } else {
    beforeEach(build);
    afterEach(dispose);
  }

  return () => {
    const built = holder;
    if (built === undefined) {
      return failEarly();
    }
    return built.value;
  };
}
