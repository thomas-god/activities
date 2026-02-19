import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { groupByClauses, metricAggregateFunctions } from '$lib/metric';
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
	workout_type: z.enum(WORKOUT_TYPE_VALUES).nullable(),
	feedback: z.string().nullable()
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
	name: z.string().nullable(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.enum(metricAggregateFunctions),
	sports: z.array(z.string()).optional(),
	workout_types: z.array(z.string()).optional(),
	bonked: z.string().nullable().optional(),
	rpes: z.array(z.string()).optional(),
	values: z.record(z.string(), z.record(z.string(), z.number())), // grouped: { group_name: { date: value } }
	group_by: z.enum(groupByClauses).nullable(),
	scope: z.discriminatedUnion('type', [
		z.object({ type: z.literal('global') }),
		z.object({ type: z.literal('trainingPeriod'), trainingPeriodId: z.string() })
	])
});

const MetricsListSchemaGrouped = z.array(MetricsListItemSchemaGrouped);

const TrainingNoteSchema = z.object({
	id: z.string(),
	title: z.string().nullable().optional(),
	content: z.string(),
	date: z.string(),
	created_at: z.string()
});

const TrainingNotesListSchema = z.array(TrainingNoteSchema);

// =============================================================================
// Types
// =============================================================================

export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItemSchema>;
export type TrainingPeriodList = z.infer<typeof TrainingPeriodListSchema>;
export type TrainingPeriodActivityItem = z.infer<typeof TrainingPeriodActivityItemSchema>;
export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetailsSchema>;
export type MetricsListItemGrouped = z.infer<typeof MetricsListItemSchemaGrouped>;
export type MetricsListGrouped = z.infer<typeof MetricsListSchemaGrouped>;
export type TrainingNote = z.infer<typeof TrainingNoteSchema>;
export type TrainingNotesList = z.infer<typeof TrainingNotesListSchema>;

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
 * Fetch a list of active training periods
 * @param fetch - The fetch function from SvelteKit
 * @param refDate - Reference date to dermine wich periods are active
 * @returns Array of training periods or empty array on error
 */
export async function fetchActiveTrainingPeriods(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	refDate: Date | string
): Promise<TrainingPeriodList> {
	const res = await fetch(
		`${PUBLIC_APP_URL}/api/training/periods/active?ref_date=${dayjs(refDate).format('YYYY-MM-DD')}`,
		{
			method: 'GET',
			mode: 'cors',
			credentials: 'include'
		}
	);

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
 * @param start - Start date for metrics
 * @param end - Optional end date for metrics
 * @param scope - Optional scope filter: 'global' for global metrics only, or { period: periodId } for period + global metrics
 * @returns Array of metrics with flat values (extracted from "no_group") or empty array on error
 */
export async function fetchTrainingMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	start: Date | string,
	end?: Date | string,
	scope?: 'global' | { period: string }
): Promise<MetricsListGrouped> {
	const params = new URLSearchParams();

	const startDate = dayjs(start).format('YYYY-MM-DDTHH:mm:ssZ');
	params.set('start', startDate);

	if (end) {
		const endDate = dayjs(end).format('YYYY-MM-DDTHH:mm:ssZ');
		params.set('end', endDate);
	}

	// Add scope parameters
	if (scope === 'global') {
		params.set('scope', 'global');
	} else if (scope && typeof scope === 'object' && 'period' in scope) {
		params.set('scope', 'period');
		params.set('period_id', scope.period);
	} else {
		// Default to global scope if not specified
		params.set('scope', 'global');
	}

	const url = `${PUBLIC_APP_URL}/api/training/metrics?${params.toString()}`;

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
		// Parse the new grouped response from the API
		const groupedMetrics = MetricsListSchemaGrouped.parse(await res.json());

		return groupedMetrics;
	}

	return [];
}

export const groupMetricValues = (metric: MetricsListItemGrouped) => {
	let values = [];
	for (const [group, time_values] of Object.entries(metric.values)) {
		for (const [dt, value] of Object.entries(time_values)) {
			values.push({ time: dt, group, value });
		}
	}
	return values;
};

export const metricScope = (metric: MetricsListItemGrouped) =>
	metric.scope.type === 'global' ? 'global' : 'local';

/**
 * Fetch all training notes for the current user
 * @param fetch - The fetch function from SvelteKit
 * @param depends - The depends function from SvelteKit loader
 * @param start - Optional start date for filtering notes
 * @param end - Optional end date for filtering notes
 * @returns Array of training notes or empty array on error
 */
export async function fetchTrainingNotes(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	depends: (...deps: `${string}:${string}`[]) => void,
	start?: Date | string,
	end?: Date | string
): Promise<TrainingNotesList> {
	depends('app:training-notes');

	const params = new URLSearchParams();

	if (start) {
		const startDate = dayjs(start).format('YYYY-MM-DDTHH:mm:ssZ');
		params.set('start', startDate);
	}

	if (end) {
		const endDate = dayjs(end).format('YYYY-MM-DDTHH:mm:ssZ');
		params.set('end', endDate);
	}

	const url = `${PUBLIC_APP_URL}/api/training/notes?${params.toString()}`;

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
		return TrainingNotesListSchema.parse(await res.json());
	}

	return [];
}

/**
 * Fetch all training notes for a specific training period
 * @param fetch - The fetch function from SvelteKit
 * @param periodId - The ID of the training period
 * @returns Array of training notes or empty array on error
 */
export async function fetchTrainingPeriodNotes(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	periodId: string
): Promise<TrainingNotesList> {
	const url = `${PUBLIC_APP_URL}/api/training/period/${periodId}/notes`;

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
		return TrainingNotesListSchema.parse(await res.json());
	}

	return [];
}

/**
 * Fetch all training metrics for a specific training period
 * @param fetch - The fetch function from SvelteKit
 * @param periodId - The ID of the training period
 * @returns Array of grouped metrics or empty array on error
 */
export async function fetchTrainingPeriodMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	periodId: string
): Promise<MetricsListGrouped> {
	const url = `${PUBLIC_APP_URL}/api/training/period/${periodId}/metrics`;

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
		return MetricsListSchemaGrouped.parse(await res.json());
	}

	return [];
}

/**
 * Create a new training note
 * @param content - The note content
 * @param date - The optional note date (defaults to today if not provided)
 * @returns The created note or null on error
 */
export async function createTrainingNote(content: string, date: string): Promise<void> {
	const body: { content: string; date: string } = {
		content,
		date
	};
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/note`, {
		method: 'POST',
		mode: 'cors',
		credentials: 'include',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify(body)
	});

	if (res.status === 401) {
		goto('/login');
	}

	return;
}

/**
 * Update an existing training note
 * @param noteId - The ID of the note to update
 * @param content - The new content
 * @param date - The date for the note
 * @returns true if successful, false otherwise
 */
export async function updateTrainingNote(
	noteId: string,
	content: string,
	date: string
): Promise<boolean> {
	const body: { content: string; date: string } = {
		content,
		date
	};
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/note/${noteId}`, {
		method: 'PATCH',
		mode: 'cors',
		credentials: 'include',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify(body)
	});

	if (res.status === 401) {
		goto('/login');
		return false;
	}

	return res.status === 204;
}

/**
 * Delete a training note
 * @param noteId - The ID of the note to delete
 * @returns true if successful, false otherwise
 */
export async function deleteTrainingNote(noteId: string): Promise<boolean> {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/note/${noteId}`, {
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
