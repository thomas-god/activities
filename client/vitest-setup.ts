import { explicit_public_env } from './.svelte-kit/generated/dev/env/config.js';
import '@testing-library/jest-dom/vitest';

// In dev, SvelteKit's generated `$app/env/public` module reads its variables
// from `globalThis.__sveltekit_dev`, which is only populated by the Vite dev
// server (and, in prod, by the client payload). Vitest runs outside both, so
// populate it manually — sourced from SvelteKit's generated env config — to
// keep tests in sync with `.env`.
(globalThis as Record<string, unknown>).__sveltekit_dev = { env: explicit_public_env };