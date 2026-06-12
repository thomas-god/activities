import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { svelteTesting } from '@testing-library/svelte/vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), svelteTesting()],
	test: {
		include: ['**/*.test.ts'],
		globals: true,
		environment: 'jsdom',
		setupFiles: ['./vitest-setup.ts']
	}
});
