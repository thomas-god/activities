import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';

// =============================================================================
// Schemas
// =============================================================================

const ActivityListItemSchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.enum(sports),
	sport_category: z.enum(SportCategories).nullable(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true })
});

const ActivityListSchema = z.array(ActivityListItemSchema);

const ActivityDetailsSchema = z.object({
	id: z.string(),
	sport: z.enum(sports),
	sport_category: z.enum(SportCategories).nullable(),
	name: z.string().nullable(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true }),
	rpe: z.number().min(1).max(10).nullable(),
	statistics: z.record(z.string(), z.number()),
	timeseries: z.object({
		time: z.array(z.number()),
		active_time: z.array(z.number().nullable()),
		metrics: z.record(
			z.string(),
			z.object({
				unit: z.string(),
				values: z.array(z.number().nullable())
			})
		),
		laps: z.array(
			z.object({
				start: z.number(),
				end: z.number()
			})
		)
	})
});

// =============================================================================
// Types
// =============================================================================

export type ActivityListItem = z.infer<typeof ActivityListItemSchema>;
export type ActivityList = z.infer<typeof ActivityListSchema>;
export type ActivityDetails = z.infer<typeof ActivityDetailsSchema>;

// =============================================================================
// API Functions
// =============================================================================

/**
 * Fetch a list of activities
 * @param fetch - The fetch function from SvelteKit
 * @param limit - Optional limit on the number of activities to fetch
 * @returns Array of activities or empty array on error
 */
export async function fetchActivities(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	limit?: number
): Promise<ActivityList> {
	const url = limit
		? `${PUBLIC_APP_URL}/api/activities?limit=${limit}`
		: `${PUBLIC_APP_URL}/api/activities`;

	const res = await fetch(url, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return [];
	}

	if (res.status === 200) {
		return ActivityListSchema.parse(await res.json());
	}

	return [];
}

/**
 * Fetch details for a specific activity
 * @param fetch - The fetch function from SvelteKit
 * @param activityId - The ID of the activity to fetch
 * @returns Activity details or null on error
 */
export async function fetchActivityDetails(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	activityId: string
): Promise<ActivityDetails | null> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activityId}`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors'
	});

	if (res.status === 401) {
		goto('/login');
		return null;
	}

	if (res.status === 200) {
		return ActivityDetailsSchema.parse(await res.json());
	}

	return null;
}
