/**
 * The public API of the geoquery TypeScript client.
 *
 * This client speaks to the engine in-process, over a versioned JSON transport defined in
 * Rust at `crates/protocol`: `{transportVersion, operation, payload}` in,
 * `{transportVersion, ok, result}` out. It builds requests and reads responses, and
 * interprets nothing.
 *
 * The types below are a hand-written mirror of the Rust DTOs, which is the one place a
 * shape is written down twice, and the reason is that the wire is JSON: a JSON wire cannot
 * be generated from Rust. What keeps the copies honest is that the same requests are
 * round-tripped through the real addon in `test/protocol.test.ts`, so a Rust field these
 * types forget fails a test rather than arriving as `undefined` at a call site.
 *
 * The one thing read from `package.json` rather than from the engine is {@link VERSION},
 * and it is read rather than hard-coded because a hand-maintained copy is a number that is
 * wrong after the first release nobody remembers to update. See `../../scripts/version.ts`.
 */

import addon from "./native.js";

const pkg = (await import("../package.json", { with: { type: "json" } })).default;

/** The client's version, and the query-protocol version it implements. */
export const VERSION: string = pkg.version;

/** The envelope version this client speaks. Must equal `TRANSPORT_VERSION` in Rust. */
export const TRANSPORT_VERSION = 1;

/** Everything the engine can be asked to do. Mirrors `Operation` in `geoquery-protocol`. */
export type Operation = "ping" | "protocolVersion" | "parseDocument";

/** One request to the engine. */
export interface EngineRequest {
  transportVersion: number;
  operation: Operation;
  payload: Record<string, unknown>;
}

/**
 * One response from the engine.
 *
 * `result` is constrained to `Record<string, unknown>` because the engine only ever
 * answers with a JSON object — a bare string or number would have no key for
 * `EngineError` to carry, and the constraint says so once instead of at every use.
 */
export interface EngineResponse<TResult extends Record<string, unknown> = Record<string, unknown>> {
  transportVersion: number;
  ok: boolean;
  result: TResult;
}

/**
 * Send one operation and return its result, throwing if the engine said no.
 *
 * What this collapses is the difference between "the engine answered, and the answer is no"
 * and "there was no answer" — both are errors for a caller, who cannot act on either. The
 * wire keeps the distinction for callers who want it, and {@link invokeRaw} returns the
 * response as it arrived.
 *
 * @throws {EngineError} if the engine reported `ok: false`.
 */
export function invoke<TResult extends Record<string, unknown> = Record<string, unknown>>(
  operation: Operation,
  payload: Record<string, unknown> = {}
): TResult {
  const response = invokeRaw<TResult>(operation, payload);
  if (!response.ok) {
    throw new EngineError(response.result);
  }
  return response.result;
}

/**
 * Send one operation and return the response as it arrived.
 *
 * For a caller that needs `ok` itself — a batch that collects failures rather than
 * stopping at the first one.
 */
export function invokeRaw<TResult extends Record<string, unknown> = Record<string, unknown>>(
  operation: Operation,
  payload: Record<string, unknown> = {}
): EngineResponse<TResult> {
  const request: EngineRequest = { transportVersion: TRANSPORT_VERSION, operation, payload };
  // The addon's own JSON.parse, with no schema check, is the honest boundary: the addon
  // returns whatever Rust serialised, and this file claims only that the two halves agree.
  return JSON.parse(addon.invoke(JSON.stringify(request))) as EngineResponse<TResult>;
}

/**
 * A failed operation, carrying the engine's own explanation.
 *
 * The message is the engine's wording rather than one composed here, so the reason a query
 * was rejected is the reason Rust gave — the same wording the CLI prints and the Python
 * SDK raises. A second phrasing in TypeScript is a second thing to keep true.
 */
export class EngineError extends Error {
  /** The failed response's `result`, for a caller that needs to branch on the reason. */
  readonly detail: Record<string, unknown>;

  constructor(detail: Record<string, unknown>) {
    super(typeof detail.error === "string" ? detail.error : JSON.stringify(detail));
    this.name = "EngineError";
    this.detail = detail;
  }
}

/**
 * Check that the addon answers, and that it echoes this client's payload back intact.
 *
 * `message` in and the same string out, because that is the shape of the smallest possible
 * round trip — the payload is `{message}`, so returning `echo` whole would hand the caller
 * an object they had to unwrap to learn the one thing they asked.
 */
export function ping(message = "typescript"): string {
  return invoke<{ echo: { message: string } }>("ping", { message }).echo.message;
}

/** This build's version, the query protocol version it speaks, and its user-agent. */
export function protocolVersion(): { version: string; protocolVersion: string; userAgent: string } {
  return invoke("protocolVersion");
}

/**
 * Validate a query document and return its sorted top-level keys.
 *
 * The same parser the CLI and the Python SDK use, reached in-process rather than over
 * HTTP. A document that is not a JSON object raises rather than returning a list.
 *
 * @throws {EngineError} if the text is not a query document.
 */
export function parseDocument(document: string): string[] {
  return invoke<{ keys: string[] }>("parseDocument", { document }).keys;
}
