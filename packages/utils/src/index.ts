/**
 * The package's public surface.
 *
 * Re-exported from one module so a consumer imports `@geoquery/utils` rather than reaching
 * into a file path, and so this file is the single place that says what the package is for.
 * Everything here is test infrastructure: nothing in `packages/geoquery`'s published output
 * imports it, and `package.json` marks it private so it cannot be published by accident.
 */

export type { Fixture, FixtureScope, Setup, Teardown } from "./fixtures.ts";
export { createFixture } from "./fixtures.ts";
