import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { WORKOUT_TYPE_VALUES } from '$lib/workout-type';
import { BONK_STATUS_VALUES } from '$lib/nutrition';
import { dayjs } from '$lib/duration';
import { resolve } from '$app/paths';

// =============================================================================
// Schemas
// =============================================================================

const NutritionSchema = z.object({
	bonk_status: z.enum(BONK_STATUS_VALUES),
	details: z.string().nullable()
});

/**
 * Canonical schema for an activity returned by the API.
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
	metrics: z.record(z.string(), z.object({ value: z.number(), unit: z.string() }))
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
 * Best rolling-average values of an activity for a fixed set of durations
 * (5s, 10s, 30s, 1min, 2min, 5min, 10min, 20min, 30min, 1h, 2h, 5h),
 * e.g. peak power or peak speed. A `null` value means the activity is too
 * short for that duration.
 */
const DurationCurveSchema = z.object({
	curve_type: z.string(),
	unit: z.string(),
	values: z.array(z.number().nullable())
});

/**
 * Additional context used to interpret or derive statistics of an activity
 * (e.g. the athlete weight at the time of the activity, used for W/kg).
 */
const TrainingContextSchema = z.object({
	weight: z.number().nullable(),
	/** Best duration curves over the 12 weeks before the activity, one per curve
	 *  type (e.g. power and pace) found in the user's history. */
	best_duration_12w_curves: z.array(DurationCurveSchema)
});

/**
 * Activity that includes its timeseries data. Structurally a superset of
 * `Activity`, so it can be used anywhere an `Activity` is expected.
 */
export const ActivityWithTimeseriesSchema = ActivitySchema.extend({
	training_context: TrainingContextSchema,
	timeseries: TimeseriesSchema,
	duration_curves: z.array(DurationCurveSchema)
});

// =============================================================================
// Types
// =============================================================================

export type Activity = z.infer<typeof ActivitySchema>;
export type ActivityList = z.infer<typeof ActivityListSchema>;
export type ActivityWithTimeseries = z.infer<typeof ActivityWithTimeseriesSchema>;
export type Timeseries = z.infer<typeof TimeseriesSchema>;
export type TrainingContext = z.infer<typeof TrainingContextSchema>;
export type DurationCurve = z.infer<typeof DurationCurveSchema>;

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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
			nbOfProcessedFiles: number;
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

			return { type: 'success', unprocessed, nbOfProcessedFiles: data.created_ids.length };
		}

		if (response.status === 401) {
			return { type: 'authentication-error' };
		}

		return { type: 'error' };
	} catch {
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
		goto(resolve('/login'));
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

export type StandaloneActivityPayload = {
	start_time: string;
	duration: number;
	sport: (typeof sports)[number];
	distance?: number;
	elevation?: number;
	calories?: number;
};

export type PostStandaloneActivityResponse =
	| { type: 'success'; id: string }
	| { type: 'duplicate' }
	| { type: 'error' }
	| { type: 'authentication-error' };

const PostStandaloneActivityResponseSchema = z.object({ id: z.string() });

export async function postStandaloneActivity(
	payload: StandaloneActivityPayload
): Promise<PostStandaloneActivityResponse> {
	try {
		const response = await fetch(`${PUBLIC_APP_URL}/api/activity/standalone`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(payload)
		});

		if (response.ok) {
			const data = PostStandaloneActivityResponseSchema.parse(await response.json());
			return { type: 'success', id: data.id };
		}

		if (response.status === 401) {
			return { type: 'authentication-error' };
		}

		if (response.status === 409) {
			return { type: 'duplicate' };
		}

		return { type: 'error' };
	} catch {
		return { type: 'error' };
	}
}

export async function fetchActivityDefaultMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<string[]> {
	const response = await fetch(`${PUBLIC_APP_URL}/api/activity/default_metrics`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' }
	});

	return await response.json();
}
