/**
 * Loading the N-API addon, and the one thing it exports.
 *
 * The addon is a shared library that Node loads through `process.dlopen`, which has two
 * consequences this file exists to absorb.
 *
 * A N-API module cannot be loaded with `import()`. It has to go through `require()`, which
 * an ESM package does not have, so the loader is built with `createRequire`. That is the
 * only place `createRequire` appears in this package: it is the one route to a native
 * module from ESM, and everything else here stays an ordinary import.
 *
 * The file name carries the platform, because `@napi-rs/cli` names the artifact after the
 * target it compiled for. It is written out literally rather than assembled from
 * `process.platform`, so that adding a target is an edit in the `napi.targets` list in
 * `package.json` and here, and a mismatch between the two fails the build that produced
 * the file rather than at an import in production.
 */

import { createRequire } from "node:module";

/** The addon's only export: the transport, in and out, as JSON text. */
export interface GeoqueryAddon {
  invoke(request: string): string;
}

// `as unknown as` because `createRequire` returns `any` and cannot know which interface
// this particular `.node` implements. What stands in for the check is `test/protocol.test.ts`
// calling every operation through the real addon, which fails if the two ever disagree.
const addon = createRequire(import.meta.url)(
  `../geoquery-node-native.linux-x64-gnu.node`,
) as unknown as GeoqueryAddon;

export default addon;