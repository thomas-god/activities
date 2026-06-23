import * as z from 'zod';
import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { SportCategories, sports, type Sport, type SportCategory } from '$lib/sport';
import {
	trainingMetricGroupByClauses,
	trainingMetricAggregateFunctions,
	trainingMetricGranularities,
	trainingMetricTemplateCategories,
	type TrainingMetricGranularity,
	type TrainingMetricAggregateFunction,
	type TrainingMetricGroupByClause
} from '$lib/trainingMetric';
import { dayjs } from '$lib/duration';
import { ActivitySchema } from './activities';
import { none, type Option, some } from '$lib/Options';
import { WORKOUT_TYPE_VALUES } from '$lib/workout-type';
import { BONK_STATUS_VALUES, type BonkStatus } from '$lib/nutrition';
import type { RPEValue } from '$lib/rpe';

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
	activities: z.array(ActivitySchema)
});

// Schema for the new API response with grouped values
const TrainingMetricSchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	metric: z.string(),
	unit: z.string(),
	scope: z.discriminatedUnion('type', [
		z.object({ type: z.literal('global') }),
		z.object({ type: z.literal('trainingPeriod'), trainingPeriodId: z.string() })
	]),
	granularity: z.enum(trainingMetricGranularities).nullable(),
	aggregate: z.enum(trainingMetricAggregateFunctions).nullable(),
	group_by: z.enum(trainingMetricGroupByClauses).nullable(),
	sports: z
		.object({
			sports: z.array(z.enum(sports)),
			categories: z.array(z.enum(SportCategories))
		})
		.nullable(),
	workout_types: z.array(z.enum(WORKOUT_TYPE_VALUES)).nullable(),
	bonked: z.enum(BONK_STATUS_VALUES).nullable().nullable(),
	rpes: z.array(z.number()).nullable(),
	show_average: z.object({ include_zeros: z.boolean() }).nullable(),
	values: z.record(z.string(), z.record(z.string(), z.number())), // grouped: { group_name: { date: value } }
	summary: z.record(z.string(), z.number())
});

const TrainingMetricListSchema = z.array(TrainingMetricSchema);

const TrainingNoteSchema = z.object({
	id: z.string(),
	title: z.string().nullable().optional(),
	content: z.string(),
	date: z.string(),
	created_at: z.string()
});

const TrainingNotesListSchema = z.array(TrainingNoteSchema);

const TrainingMetricTemplatesSchema = z.array(
	z.object({
		display_name: z.string(),
		metric: z.string(),
		aggregate: z.enum(trainingMetricAggregateFunctions),
		unit: z.string(),
		category: z.enum(trainingMetricTemplateCategories)
	})
);

const TrainingMetricValuesPreviewSchema = z.object({
	values: z.record(z.string(), z.record(z.string(), z.number())), // grouped: { group_name: { date: value } }
	summary: z.record(z.string(), z.number()),
	unit: z.string()
});

// =============================================================================
// Types
// =============================================================================

export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItemSchema>;
export type TrainingPeriodList = z.infer<typeof TrainingPeriodListSchema>;
export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetailsSchema>;
export type TrainingMetric = z.infer<typeof TrainingMetricSchema>;
export type TrainingMetricList = z.infer<typeof TrainingMetricListSchema>;
export type TrainingNote = z.infer<typeof TrainingNoteSchema>;
export type TrainingNotesList = z.infer<typeof TrainingNotesListSchema>;
export type TrainingMetricTemplate = z.infer<typeof TrainingMetricTemplatesSchema>[number];
export type TrainingMetricValuesPreview = z.infer<typeof TrainingMetricValuesPreviewSchema>;

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
 * @param end - End date for metrics
 * @param scope - Optional scope filter: 'global' for global metrics only, or { period: periodId } for period + global metrics
 * @returns Array of metrics with flat values (extracted from "no_group") or empty array on error
 */
export async function fetchTrainingMetrics(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	start: Date | string,
	end: Date | string,
	scope?: 'global' | { period: string }
): Promise<TrainingMetricList> {
	const params = new URLSearchParams();

	const startDate = dayjs(start).format('YYYY-MM-DDTHH:mm:ssZ');
	params.set('start', startDate);

	const endDate = dayjs(end).format('YYYY-MM-DDTHH:mm:ssZ');
	params.set('end', endDate);

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
		const groupedMetrics = TrainingMetricListSchema.parse(await res.json());

		return groupedMetrics;
	}

	return [];
}

/**
 * Fetch training metrics
 * @param sourceMetric - ID of the metric to copy
 * @param targetPeriod - ID of the training period to copy the metric into
 * @param newName - Name of the new metric
 */
export async function copyTrainingMetricIntoPeriod(
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	sourceMetric: string,
	targetPeriod: string,
	newName: string | null
): Promise<void> {
	let body: Record<string, string> = { targetPeriod };
	if (newName !== null) {
		body = { newName, ...body };
	}

	const url = `${PUBLIC_APP_URL}/api/training/metric/${sourceMetric}/copy`;

	const res = await fetch(url, {
		method: 'POST',
		mode: 'cors',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

	if (res.status === 401) {
		goto('/login');
		return;
	}

	if (res.status === 201) {
		return;
	}

	return;
}

export const groupMetricValues = (metric: TrainingMetric) => {
	let values = [];
	for (const [group, time_values] of Object.entries(metric.values)) {
		for (const [dt, value] of Object.entries(time_values)) {
			values.push({ time: dt, group, value });
		}
	}
	return values;
};

export const metricScope = (metric: TrainingMetric) =>
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
): Promise<TrainingMetricList> {
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
		return TrainingMetricListSchema.parse(await res.json());
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

export const fetchTrainingMetricTemplates = async () => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics/templates`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	return TrainingMetricTemplatesSchema.parse(await res.json());
};

export interface TrainingMetricBasePayload {
	metric: string;
	window?: {
		granularity: TrainingMetricGranularity;
		aggregate: TrainingMetricAggregateFunction;
		group_by?: TrainingMetricGroupByClause;
	};
	filters?: {
		sports?: (
			| {
					Sport: Sport;
			  }
			| { SportCategory: SportCategory }
		)[];
		workout_types?: WorkerType[];
		rpes?: RPEValue[];
		bonked?: BonkStatus;
	};
	summary?: {
		average: {
			include_zeros: boolean;
		};
	};
}

export interface PreviewTrainingMetricPayload extends TrainingMetricBasePayload {
	start: string;
	end: string;
}

export interface UpdateTrainingMetricPayload extends TrainingMetricBasePayload {
	name: string;
}

export interface CreateTrainingMetricPayload extends TrainingMetricBasePayload {
	name: string;
	scope: { type: 'global' } | { type: 'trainingPeriod'; trainingPeriodId: string };
}

export const createTrainingMetric = async (payload: CreateTrainingMetricPayload) => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric`, {
		body: JSON.stringify(payload),
		method: 'POST',
		credentials: 'include',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' }
	});

	if (res.status === 401) {
		goto('/login');
	}
};

export const updateTrainingMetric = async (
	metric: string,
	payload: UpdateTrainingMetricPayload
) => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric}`, {
		body: JSON.stringify(payload),
		method: 'PATCH',
		credentials: 'include',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' }
	});

	if (res.status === 401) {
		goto('/login');
	}
};

export const getTrainingMetricPreview = async (
	payload: PreviewTrainingMetricPayload
): Promise<Option<TrainingMetricValuesPreview>> => {
	const body = JSON.stringify(payload);
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/values`, {
		body,
		method: 'POST',
		credentials: 'include',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' }
	});

	if (res.status === 401) {
		goto('/login');
	}

	if (res.status !== 200) {
		return none();
	}

	return some(TrainingMetricValuesPreviewSchema.parse(await res.json()));
};
