/**
 * The version of this package, and of the language it speaks.
 *
 * Both numbers are the same number: the query protocol is versioned with the
 * implementation, so a client that reports its own version is reporting the protocol
 * version it was built against. The value is injected rather than written in — see
 * `../../scripts/version.ts` — because a hand-maintained copy is a number that is wrong
 * after the first release nobody remembers to update.
 */

/**
 * The package version, read at build time from `package.json`.
 *
 * `import ... with { type: "json" }` rather than `require`, because the package is ESM
 * (`"type": "module"`) and `verbatimModuleSyntax` is on: this module is loaded by bun
 * during tests and by tsc during type-checking, and an import attribute works in both
 * where `createRequire` does not.
 */
const pkg = (await import("../package.json", { with: { type: "json" } })).default;

/** The client's version, and the query-protocol version it implements. */
export const VERSION: string = pkg.version;

/**
 * The user-agent a geoquery service should see from this client.
 *
 * A protocol implementation that does not identify itself is indistinguishable from a
 * browser full of scripts, and the first question an operator asks about a slow or
 * failing query is who sent it.
 */
export const USER_AGENT: string = `geoquery-client/${VERSION}`;
