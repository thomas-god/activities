import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { metricAggregateFunctions } from '$lib/metric';
import { dayjs } from '$lib/duration';
import { WORKOUT_TYPE_VALUES } from '$lib/workout-type';

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
	start_time: z.string(),
	rpe: z.number().min(1).max(10).nullable(),
	workout_type: z.enum(WORKOUT_TYPE_VALUES).nullable()
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

// Schema for the new API response with grouped values
const MetricsListItemSchemaGrouped = z.object({
	id: z.string(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.enum(metricAggregateFunctions),
	sports: z.array(z.string()).optional(),
	values: z.record(z.string(), z.record(z.string(), z.number())), // grouped: { group_name: { date: value } }
	group_by: z.string().nullable()
});

// Legacy schema for backward compatibility (flat values)
const MetricsListItemSchema = z.object({
	id: z.string(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.enum(metricAggregateFunctions),
	sports: z.array(z.string()).optional(),
	values: z.record(z.string(), z.number())
});

const MetricsListSchemaGrouped = z.array(MetricsListItemSchemaGrouped);
const MetricsListSchema = z.array(MetricsListItemSchema);

// =============================================================================
// Types
// =============================================================================

export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItemSchema>;
export type TrainingPeriodList = z.infer<typeof TrainingPeriodListSchema>;
export type TrainingPeriodActivityItem = z.infer<typeof TrainingPeriodActivityItemSchema>;
export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetailsSchema>;
export type MetricsListItemGrouped = z.infer<typeof MetricsListItemSchemaGrouped>;
export type MetricsListItem = z.infer<typeof MetricsListItemSchema>;
export type MetricsList = z.infer<typeof MetricsListSchema>;
export type MetricsListGrouped = z.infer<typeof MetricsListSchemaGrouped>;

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Extract "no_group" values from the new grouped API response and convert to flat structure
 * This maintains backward compatibility with existing chart components
 * @param groupedMetric - Metric with grouped values from the API
 * @returns Metric with flat values (only "no_group" data)
 */
export function extractNoGroupValues(groupedMetric: MetricsListItemGrouped): MetricsListItem {
	const noGroupValues = groupedMetric.values['no_group'] ?? {};

	return {
		id: groupedMetric.id,
		metric: groupedMetric.metric,
		unit: groupedMetric.unit,
		granularity: groupedMetric.granularity,
		aggregate: groupedMetric.aggregate,
		sports: groupedMetric.sports,
		values: noGroupValues
	};
}

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
 * @returns Array of metrics with flat values (extracted from "no_group") or empty array on error
 */
export async function fetchTrainingMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	start: Date | string,
	end?: Date | string
): Promise<{ noGroup: MetricsList; metrics: MetricsListGrouped }> {
	const startDate = dayjs(start).format('YYYY-MM-DDTHH:mm:ssZ');

	let url = `${PUBLIC_APP_URL}/api/training/metrics?start=${encodeURIComponent(startDate)}`;

	if (end) {
		const endDate = dayjs(end).format('YYYY-MM-DDTHH:mm:ssZ');
		url += `&end=${encodeURIComponent(endDate)}`;
	}

	const res = await fetch(url, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto('/login');
		return { noGroup: [], metrics: [] };
	}

	if (res.status === 200) {
		// Parse the new grouped response from the API
		const groupedMetrics = MetricsListSchemaGrouped.parse(await res.json());

		// Extract "no_group" values and convert to flat structure for backward compatibility
		return { noGroup: groupedMetrics.map(extractNoGroupValues), metrics: groupedMetrics };
	}

	return { noGroup: [], metrics: [] };
}
