import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { metricAggregateFunctions } from '$lib/metric';
import { dayjs } from '$lib/duration';

// =============================================================================
// Schemas
// =============================================================================

const TrainingPeriodListItemSchema = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.object({
		sports: z.array(z.enum(sports)),
		categories: z.array(z.enum(SportCategories))
	}),
	note: z.string().nullable()
});

const TrainingPeriodListSchema = z.array(TrainingPeriodListItemSchema);

const TrainingPeriodActivityItemSchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.enum(sports),
	sport_category: z.enum(SportCategories).nullable(),
	duration: z
		.number()
		.nullable()
		.transform((val) => val ?? 0), // Transform null to 0 for compatibility
	distance: z.number().nullable(),
	elevation: z.number().nullable(),
	start_time: z.string()
});

const TrainingPeriodDetailsSchema = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.object({
		sports: z.array(z.enum(sports)),
		categories: z.array(z.enum(SportCategories))
	}),
	note: z.string().nullable(),
	activities: z.array(TrainingPeriodActivityItemSchema)
});

const MetricsListItemSchema = z.object({
	id: z.string(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.enum(metricAggregateFunctions),
	sports: z.array(z.string()).optional(),
	values: z.record(z.string(), z.number())
});

const MetricsListSchema = z.array(MetricsListItemSchema);

// =============================================================================
// Types
// =============================================================================

export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItemSchema>;
export type TrainingPeriodList = z.infer<typeof TrainingPeriodListSchema>;
export type TrainingPeriodActivityItem = z.infer<typeof TrainingPeriodActivityItemSchema>;
export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetailsSchema>;
export type MetricsListItem = z.infer<typeof MetricsListItemSchema>;
export type MetricsList = z.infer<typeof MetricsListSchema>;

// =============================================================================
// API Functions
// =============================================================================

/**
 * Fetch a list of training periods
 * @param fetch - The fetch function from SvelteKit
 * @returns Array of training periods or empty array on error
 */
export async function fetchTrainingPeriods(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<TrainingPeriodList> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/periods`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return [];
	}

	if (res.status === 200) {
		return TrainingPeriodListSchema.parse(await res.json());
	}

	return [];
}

/**
 * Fetch details for a specific training period including its activities
 * @param fetch - The fetch function from SvelteKit
 * @param periodId - The ID of the training period to fetch
 * @returns Training period details or null on error
 */
export async function fetchTrainingPeriodDetails(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	periodId: string
): Promise<TrainingPeriodDetails | null> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors'
	});

	if (res.status === 401) {
		goto('/login');
		return null;
	}

	if (res.status === 200) {
		return TrainingPeriodDetailsSchema.parse(await res.json());
	}

	return null;
}

/**
 * Fetch training metrics
 * @param fetch - The fetch function from SvelteKit
 * @param start - Optional start date for metrics (defaults to 3 weeks ago)
 * @param end - Optional end date for metrics
 * @returns Array of metrics or empty array on error
 */
export async function fetchTrainingMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	start?: Date | string,
	end?: Date | string
): Promise<MetricsList> {
	// Default to 3 weeks ago if no start date provided
	const startDate = start
		? typeof start === 'string'
			? start
			: dayjs(start).format('YYYY-MM-DDTHH:mm:ssZ')
		: dayjs().startOf('isoWeek').subtract(3, 'week').format('YYYY-MM-DDTHH:mm:ssZ');

	let url = `${PUBLIC_APP_URL}/api/training/metrics?start=${encodeURIComponent(startDate)}`;

	if (end) {
		const endDate = typeof end === 'string' ? end : dayjs(end).format('YYYY-MM-DDTHH:mm:ssZ');
		url += `&end=${encodeURIComponent(endDate)}`;
	}

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
		return MetricsListSchema.parse(await res.json());
	}

	return [];
}
