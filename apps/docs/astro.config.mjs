// @ts-check
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

import { workspaceVersion } from "../../scripts/version.ts";

/**
 * The documented version is decided by the environment that builds the site, never
 * written into it. `pixi run docs-build` exports GEOQUERY_VERSION from the one manifest
 * that owns the number; the fallback reads the same manifest directly, so a build
 * started with a bare `bun run build` still shows the real number instead of a
 * placeholder that is wrong after the first release.
 */
const version = process.env.GEOQUERY_VERSION ?? workspaceVersion();

/**
 * `site` and `base` are the project-pages URL this deploys to
 * (https://archont561.github.io/geoquery). Both are required rather than optional: with
 * `base` missing, every internal link and asset would be absolute from the domain root,
 * which is another project's site.
 */
export default defineConfig({
  site: "https://archont561.github.io",
  base: "/geoquery",
  integrations: [
    starlight({
      title: `geoquery ${version}`,
      description:
        "One protocol-independent query language and execution engine for federating " +
        "geospatial resources and services.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/Archont561/geoquery"
        }
      ],
      editLink: {
        baseUrl: "https://github.com/Archont561/geoquery/edit/main/apps/docs/"
      },
      sidebar: [
        { label: "Start here", items: [{ label: "What geoquery is", slug: "index" }] },
        {
          label: "Using it",
          items: [
            { label: "The command line", slug: "cli" },
            { label: "Working offline", slug: "offline" }
          ]
        }
      ]
    })
  ]
});
