import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { WORKOUT_TYPE_VALUES } from '$lib/workout-type';
import { BONK_STATUS_VALUES } from '$lib/nutrition';
import { dayjs } from '$lib/duration';

// =============================================================================
// Schemas
// =============================================================================

const NutritionSchema = z.object({
	bonk_status: z.enum(BONK_STATUS_VALUES),
	details: z.string().nullable()
});

/**
 * Canonical schema for an activity returned by the API.
 * All quantity fields (duration, distance, elevation, …) are surfaced through
 * `statistics` so that every endpoint returns an identical shape.
 */
export const ActivitySchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.enum(sports),
	sport_category: z.enum(SportCategories).nullable(),
	start_time: z.iso.datetime({ offset: true }),
	rpe: z.number().min(1).max(10).nullable(),
	workout_type: z.enum(WORKOUT_TYPE_VALUES).nullable(),
	feedback: z.string().nullable(),
	nutrition: NutritionSchema.nullable(),
	statistics: z.record(z.string(), z.number())
});

const ActivityListSchema = z.array(ActivitySchema);

const TimeseriesSchema = z.object({
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
});

/**
 * Activity that includes its timeseries data. Structurally a superset of
 * `Activity`, so it can be used anywhere an `Activity` is expected.
 */
const ActivityWithTimeseriesSchema = ActivitySchema.extend({
	timeseries: TimeseriesSchema
});

// =============================================================================
// Types
// =============================================================================

export type Activity = z.infer<typeof ActivitySchema>;
export type ActivityList = z.infer<typeof ActivityListSchema>;
export type ActivityWithTimeseries = z.infer<typeof ActivityWithTimeseriesSchema>;
export type Timeseries = z.infer<typeof TimeseriesSchema>;

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
	limit?: number,
	start?: Date | string,
	end?: Date | string
): Promise<ActivityList> {
	const params = new URLSearchParams();

	if (start) {
		const startDate = dayjs(start).format('YYYY-MM-DD');
		params.set('start_date', startDate);
	}
	if (end) {
		const endDate = dayjs(end).format('YYYY-MM-DD');
		params.set('end_date', endDate);
	}
	if (limit) {
		params.set('limit', limit.toString());
	}

	const url = `${PUBLIC_APP_URL}/api/activities?${params.toString()}`;
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
 * Fetch details for a specific activity (includes timeseries)
 * @param fetch - The fetch function from SvelteKit
 * @param activityId - The ID of the activity to fetch
 * @returns Activity with timeseries or null on error
 */
export async function fetchActivityDetails(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	activityId: string
): Promise<ActivityWithTimeseries | null> {
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
		return ActivityWithTimeseriesSchema.parse(await res.json());
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
