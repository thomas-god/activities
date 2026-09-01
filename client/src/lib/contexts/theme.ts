import { browser } from '$app/environment';
import { createContext } from 'svelte';

// Make sure corresponding themes are enable in `app.css` daisyui's plugin
const LIGHT_THEME = 'nord';
const DARK_THEME = 'dim';

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

		document.documentElement.setAttribute('data-theme', themeName(newTheme.variant));
	}
};

const themeName = (variant: Theme['variant']): string => {
	if (variant === 'dark') {
		return DARK_THEME;
	} else {
		return LIGHT_THEME;
	}
};
