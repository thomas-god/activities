import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	test: {
		include: ['**/*.test.ts'],
		globals: true,
		environment: 'jsdom',
		setupFiles: ['./vitest-setup.ts']
	}
});
