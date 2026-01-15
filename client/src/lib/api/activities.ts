import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { WORKOUT_TYPE_VALUES } from '$lib/workout-type';
import { BONK_STATUS_VALUES } from '$lib/nutrition';

// =============================================================================
// Schemas
// =============================================================================

const NutritionSchema = z.object({
	bonk_status: z.enum(BONK_STATUS_VALUES),
	details: z.string().nullable()
});

const ActivityListItemSchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.enum(sports),
	sport_category: z.enum(SportCategories).nullable(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true }),
	rpe: z.number().min(1).max(10).nullable(),
	workout_type: z.enum(WORKOUT_TYPE_VALUES).nullable(),
	feedback: z.string().nullable()
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
	workout_type: z.enum(WORKOUT_TYPE_VALUES).nullable(),
	nutrition: NutritionSchema.nullable(),
	feedback: z.string().nullable(),
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

const PostActivitiesResponseSchema = z.object({
	created_ids: z.array(z.string()),
	unprocessable_files: z.array(
		z.tuple([
			z.string(),
			z.enum([
				'CannotReadContent',
				'CannotProcessFile',
				'DuplicatedActivity',
				'IncoherentTimeseries',
				'UnsupportedFileExtension',
				'Unknown'
			])
		])
	)
});

export type PostActivitiesResponse =
	| {
			type: 'success';
			unprocessed: { file: string; reason: 'duplicated' | 'invalid' }[];
	  }
	| {
			type: 'error';
	  }
	| { type: 'authentication-error' };

export async function postActivities(body: FormData): Promise<PostActivitiesResponse> {
	try {
		const response = await fetch(`${PUBLIC_APP_URL}/api/activity`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			body
		});

		if (response.ok) {
			const data = PostActivitiesResponseSchema.parse(await response.json());
			const unprocessed: { file: string; reason: 'duplicated' | 'invalid' }[] =
				data.unprocessable_files.map(([file, reason]) => {
					const mappedReason = reason === 'DuplicatedActivity' ? 'duplicated' : 'invalid';
					return { file, reason: mappedReason };
				});

			return { type: 'success', unprocessed };
		}

		if (response.status === 401) {
			return { type: 'authentication-error' };
		}

		return { type: 'error' };
	} catch (error) {
		return { type: 'error' };
	}
}

/**
 * Download all activities as a ZIP file
 * @returns Promise that resolves when download is complete or rejects on error
 */
export async function downloadAllActivities(): Promise<void> {
	const response = await fetch(`${PUBLIC_APP_URL}/api/activities/download`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors'
	});

	if (response.status === 401) {
		goto('/login');
		throw new Error('Unauthorized');
	}

	if (!response.ok) {
		throw new Error('Failed to download activities');
	}

	// Get the blob from the response
	const blob = await response.blob();

	// Create a temporary URL for the blob
	const url = window.URL.createObjectURL(blob);

	// Create a temporary anchor element and trigger download
	const a = document.createElement('a');
	a.href = url;
	a.download = 'activities.zip';
	document.body.appendChild(a);
	a.click();

	// Clean up
	window.URL.revokeObjectURL(url);
	document.body.removeChild(a);
}
