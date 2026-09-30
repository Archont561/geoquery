import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

/**
 * Starlight reads its pages through the content layer, so the collection has to be
 * declared even though the loader and the schema are both the defaults: without this
 * file, `src/content/docs/` is an ordinary directory and every page 404s at build time.
 */
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() })
};
