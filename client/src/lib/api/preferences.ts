import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { none, some, type Option } from '$lib/Options';
import { resolve } from '$app/paths';

// =============================================================================
// Schemas
// =============================================================================

export const PreferenceSchema = z.discriminatedUnion('key', [
	z.object({
		key: z.literal('favorite_metric'),
		value: z.string()
	}),
	z.object({
		key: z.literal('activity_list_summary'),
		value: z.object({
			scope: z.discriminatedUnion('type', [
				z.object({ type: z.literal('global') }),
				z.object({ type: z.literal('trainingPeriod'), trainingPeriodId: z.string() })
			]),
			items: z.array(
				z.discriminatedUnion('type', [
					z.object({ type: z.literal('metric'), value: z.string() }),
					z.object({ type: z.literal('rpe') }),
					z.object({ type: z.literal('workoutType') })
				])
			)
		})
	})
]);

export const PreferencesListSchema = z.array(PreferenceSchema);

// =============================================================================
// Types
// =============================================================================

export type PreferencePayload = z.infer<typeof PreferenceSchema>;
export type PreferencesList = z.infer<typeof PreferencesListSchema>;

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
		goto(resolve('/login'));
		return [];
	}

	if (res.status === 200) {
		return PreferencesListSchema.parse(await res.json());
	}

	return [];
}

export type ActivityListSummaryItems = Extract<
	PreferencePayload,
	{ key: 'activity_list_summary' }
>['value']['items'];

export const fetchActivityListSummary = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<ActivityListSummaryItems> => {
	const preferences = await fetchAllPreferences(fetch);
	const preference = preferences.find((pref) => pref.key === 'activity_list_summary');
	if (preference === undefined) {
		// Default activity list summary
		return [{ type: 'workoutType' }, { type: 'rpe' }, { type: 'metric', value: 'Duration' }];
	}
	return preference.value.items;
};

/**
 * Fetch a specific preference by key
 * @param fetch - The fetch function from SvelteKit
 * @param key - The preference key to fetch
 * @returns The preference or null if not found/error
 */
export async function fetchPreference(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	key: PreferencePayload['key']
): Promise<Option<PreferencePayload>> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/preferences/${key}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto(resolve('/login'));
		return none();
	}

	if (res.status === 200) {
		const data = await res.json();
		if (data === null) {
			return none();
		}
		return some(PreferenceSchema.parse(data));
	}

	return none();
}

/**
 * Set (create or update) a preference
 * @param fetch - The fetch function from SvelteKit
 * @param preference - The preference to set
 * @returns true if successful, false otherwise
 */
export async function setPreference(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	preference: PreferencePayload
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
		return false;
	}

	return res.status === 204;
}
