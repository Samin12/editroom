/**
 * Compositions for `remotion studio` (job.mjs studio regenerates this file in the runtime
 * workspace). Renders and stills never use it: each job gets its own entry file under
 * src/entries/, so concurrent renders never share a registry and one broken scene never breaks
 * another job's bundle.
 */
import type { JobEntry } from "./types";

export const jobs: JobEntry[] = [];
