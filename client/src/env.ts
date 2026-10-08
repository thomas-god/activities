import { defineEnvVars } from '@sveltejs/kit/env';

export const variables = defineEnvVars({ PUBLIC_APP_URL: { public: true, static: true } });
