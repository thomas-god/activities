import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';

// =============================================================================
// Schemas
// =============================================================================

export const PreferenceResponseSchema = z.discriminatedUnion('key', [
	z.object({
		key: z.literal('favorite_metric'),
		value: z.string()
	})
]);

export const PreferencesListSchema = z.array(PreferenceResponseSchema);

// =============================================================================
// Types
// =============================================================================

export type PreferenceResponse = z.infer<typeof PreferenceResponseSchema>;
export type PreferencesList = z.infer<typeof PreferencesListSchema>;

export type SetPreferenceRequest = {
	key: 'favorite_metric';
	value: string;
};

// =============================================================================
// API Functions
// =============================================================================

/**
 * Fetch all preferences for the current user
 * @param fetch - The fetch function from SvelteKit
 * @returns Array of preferences or empty array on error
 */
export async function fetchAllPreferences(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<PreferencesList> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/preferences`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return [];
	}

	if (res.status === 200) {
		return PreferencesListSchema.parse(await res.json());
	}

	return [];
}

/**
 * Fetch a specific preference by key
 * @param fetch - The fetch function from SvelteKit
 * @param key - The preference key to fetch
 * @returns The preference or null if not found/error
 */
export async function fetchPreference(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	key: string
): Promise<PreferenceResponse | null> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/preferences/${key}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return null;
	}

	if (res.status === 200) {
		const data = await res.json();
		if (data === null) {
			return null;
		}
		return PreferenceResponseSchema.parse(data);
	}

	return null;
}

/**
 * Set (create or update) a preference
 * @param fetch - The fetch function from SvelteKit
 * @param preference - The preference to set
 * @returns true if successful, false otherwise
 */
export async function setPreference(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	preference: SetPreferenceRequest
): Promise<boolean> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/preferences`, {
		method: 'POST',
		mode: 'cors',
		credentials: 'include',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify(preference)
	});

	if (res.status === 401) {
		goto('/login');
		return false;
	}

	return res.status === 204;
}

/**
 * Delete a preference by key
 * @param fetch - The fetch function from SvelteKit
 * @param key - The preference key to delete
 * @returns true if successful, false otherwise
 */
export async function deletePreference(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	key: string
): Promise<boolean> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/preferences/${key}`, {
		method: 'DELETE',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return false;
	}

	return res.status === 204;
}
