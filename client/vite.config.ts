import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/// <reference types="vitest/config" />
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { svelteTesting } from '@testing-library/svelte/vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			// Consult https://svelte.dev/docs/kit/integrations
			// for more information about preprocessors
			preprocess: vitePreprocess(),
			compilerOptions: { experimental: { async: true } },
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				fallback: 'index.html',
				precompress: false,
				strict: true
			}),
			prerender: { handleHttpError: 'fail' },
			alias: { $components: './src/components', $ui: './src/ui' }
		}),
		svelteTesting()
	],
	server: { host: true, port: 5173 },
	test: {
		include: ['**/*.test.ts'],
		globals: true,
		environment: 'jsdom',
		setupFiles: ['./vitest-setup.ts']
	}
});
