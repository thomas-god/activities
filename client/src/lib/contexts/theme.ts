import { browser } from '$app/environment';
import { createContext } from 'svelte';

export interface Theme {
	variant: 'dark' | 'light';
}

export const [getTheme, setTheme] = createContext<Theme>();

export const loadTheme = (): Theme => {
	if (!browser) {
		return { variant: 'light' };
	}

	if (localStorage !== undefined) {
		const stored = localStorage.getItem('theme');
		if (stored !== null && (stored === 'light' || stored === 'dark')) {
			return { variant: stored };
		}
	}

	if (window !== undefined) {
		const preference = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
		return { variant: preference };
	}

	return { variant: 'light' };
};

export const persistTheme = (newTheme: Theme) => {
	if (browser) {
		localStorage.setItem('theme', newTheme.variant);
		document.documentElement.setAttribute('data-theme', newTheme.variant);
	}
};
