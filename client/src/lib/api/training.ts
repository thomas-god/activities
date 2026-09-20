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
import { resolve } from '$app/paths';

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

const TrainingMetricSourceSchema = z.discriminatedUnion('type', [
	z.object({ type: z.literal('activity'), metric: z.object({ metric: z.string() }) }),
	z.object({ type: z.literal('hooperIndex'), metric: z.string() }),
	z.object({ type: z.literal('weightAndNutrition'), metric: z.string() })
]);

// Schema for the new API response with grouped values
const TrainingMetricSchema = z.object({
	id: z.string(),
	name: z.string().nullable(),
	source: TrainingMetricSourceSchema,
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
	target: z.object({ value: z.number(), unit: z.string() }).nullable(),
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

const TemplateMetricSourceSchema = z.discriminatedUnion('type', [
	z.object({ type: z.literal('activity'), metric: z.string() }),
	z.object({ type: z.literal('hooperIndex'), metric: z.string() }),
	z.object({ type: z.literal('weightAndNutrition'), metric: z.string() })
]);

const TrainingMetricTemplatesSchema = z.array(
	z.object({
		display_name: z.string(),
		source: TemplateMetricSourceSchema,
		aggregate: z.enum(trainingMetricAggregateFunctions),
		unit: z.string(),
		category: z.enum(trainingMetricTemplateCategories)
	})
);

/** A subjective Hooper index measure, constrained to the 0..=10 range. */
const SubjectiveScaleSchema = z.number().int().min(0).max(10);

export const HooperIndexSchema = z.object({
	fatigue: SubjectiveScaleSchema.nullable(),
	sleep: SubjectiveScaleSchema.nullable(),
	pain: SubjectiveScaleSchema.nullable(),
	stress: SubjectiveScaleSchema.nullable(),
	mood: SubjectiveScaleSchema.nullable()
});

export const CreateHooperIndexSchema = HooperIndexSchema.extend({
	date: z.string()
});

/**
 * Patch body for a Hooper index. Mirrors the API patch semantics:
 * - an omitted field leaves the current value untouched,
 * - `null` clears the value,
 * - a number sets it.
 */
export const UpdateHooperIndexSchema = z.object({
	fatigue: SubjectiveScaleSchema.nullable().optional(),
	sleep: SubjectiveScaleSchema.nullable().optional(),
	pain: SubjectiveScaleSchema.nullable().optional(),
	stress: SubjectiveScaleSchema.nullable().optional(),
	mood: SubjectiveScaleSchema.nullable().optional()
});

/** Weight, body composition, nutrition and hydration measures for a given date. */
export const WeightAndNutritionSchema = z.object({
	weight: z.number().nullable(),
	fat: z.number().nullable(),
	muscle: z.number().nullable(),
	bmi: z.number().nullable(),
	calories: z.number().nullable(),
	lipid: z.number().nullable(),
	carbs: z.number().nullable(),
	protein: z.number().nullable(),
	water: z.number().nullable(),
	alcohol: z.number().nullable()
});

export const CreateWeightAndNutritionSchema = WeightAndNutritionSchema.extend({
	date: z.string()
});

/**
 * Patch body for weight and nutrition. Mirrors the API patch semantics:
 * - an omitted field leaves the current value untouched,
 * - `null` clears the value,
 * - a number sets it.
 */
export const UpdateWeightAndNutritionSchema = z.object({
	weight: z.number().nullable().optional(),
	fat: z.number().nullable().optional(),
	muscle: z.number().nullable().optional(),
	bmi: z.number().nullable().optional(),
	calories: z.number().nullable().optional(),
	lipid: z.number().nullable().optional(),
	carbs: z.number().nullable().optional(),
	protein: z.number().nullable().optional(),
	water: z.number().nullable().optional(),
	alcohol: z.number().nullable().optional()
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
export type HooperIndex = z.infer<typeof HooperIndexSchema>;
export type CreateHooperIndexBody = z.infer<typeof CreateHooperIndexSchema>;
export type UpdateHooperIndexPatch = z.infer<typeof UpdateHooperIndexSchema>;
export type WeightAndNutrition = z.infer<typeof WeightAndNutritionSchema>;
export type CreateWeightAndNutritionBody = z.infer<typeof CreateWeightAndNutritionSchema>;
export type UpdateWeightAndNutritionPatch = z.infer<typeof UpdateWeightAndNutritionSchema>;

// =============================================================================
// Helper Functions
// =============================================================================
//
export const metricAsString = (metric: TrainingMetric): string => {
	if (metric.source.type === 'activity') {
		return metric.source.metric.metric.toLocaleLowerCase();
	} else {
		return metric.source.metric.toLocaleLowerCase();
	}
};

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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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

	const startDate = dayjs(start).format('YYYY-MM-DD');
	params.set('start', startDate);

	const endDate = dayjs(end).format('YYYY-MM-DD');
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
		return;
	}

	if (res.status === 201) {
		return;
	}

	return;
}

export const groupMetricValues = (metric: TrainingMetric) => {
	const values = [];
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
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
	source:
		| {
				type: 'hooperIndex' | 'weightAndNutrition';
				metric: string;
		  }
		| { type: 'activity'; metric: { metric: string } };
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
	target?: {
		value: number;
		unit: string;
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
		goto(resolve('/login'));
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
		goto(resolve('/login'));
	}
};

export const getTrainingMetricPreview = async (
	payload: PreviewTrainingMetricPayload
): Promise<Option<TrainingMetric>> => {
	const body = JSON.stringify({
		...payload,
		start: dayjs(payload.start).format('YYYY-MM-DD'),
		end: dayjs(payload.end).format('YYYY-MM-DD')
	});
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/values`, {
		body,
		method: 'POST',
		credentials: 'include',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' }
	});

	if (res.status === 401) {
		goto(resolve('/login'));
	}

	if (res.status !== 200) {
		return none();
	}

	return some(TrainingMetricSchema.parse(await res.json()));
};

// =============================================================================
// Hooper index
// =============================================================================

/**
 * Fetch the Hooper index for a given date.
 *
 * A date without a stored index resolves to an index with every measure unset, so callers can
 * always rely on the same response shape.
 * @param date - The date the index applies to
 * @returns The Hooper index for that date
 */
export async function fetchHooperIndex(date: Date | string): Promise<HooperIndex> {
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/hooper-index/${formattedDate}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto(resolve('/login'));
		return { fatigue: null, sleep: null, pain: null, stress: null, mood: null };
	}

	if (res.status !== 200) {
		throw new Error(`Failed to fetch the Hooper index: ${res.status}`);
	}

	return HooperIndexSchema.parse(await res.json());
}

/**
 * Save the Hooper index for a given date.
 * @param date - The date the index applies to
 * @param patch - The measures to save: omitted fields are left untouched, `null` clears them
 * @returns true if the index was updated, false otherwise
 */
export async function saveHooperIndex(
	date: Date | string,
	patch: UpdateHooperIndexPatch
): Promise<boolean> {
	const body = UpdateHooperIndexSchema.parse(patch);
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/hooper-index/${formattedDate}`, {
		method: 'PATCH',
		mode: 'cors',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

	if (res.status === 401) {
		goto(resolve('/login'));
		return false;
	}

	return res.status === 204;
}

/**
 * Delete the Hooper index for a given date.
 * @param date - The date the index applies to
 * @returns true if the index was deleted, false otherwise
 */
export async function deleteHooperIndex(date: Date | string): Promise<boolean> {
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/hooper-index/${formattedDate}`, {
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

// =============================================================================
// Weight and nutrition
// =============================================================================

/**
 * Fetch the weight and nutrition values for a given date.
 *
 * A date without stored values resolves to an entry with every measure unset, so callers can
 * always rely on the same response shape.
 * @param date - The date the values apply to
 * @returns The weight and nutrition values for that date
 */
export async function fetchWeightAndNutrition(date: Date | string): Promise<WeightAndNutrition> {
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/weight-and-nutrition/${formattedDate}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});

	if (res.status === 401) {
		goto(resolve('/login'));
		return {
			weight: null,
			fat: null,
			muscle: null,
			bmi: null,
			calories: null,
			lipid: null,
			carbs: null,
			protein: null,
			water: null,
			alcohol: null
		};
	}

	if (res.status !== 200) {
		throw new Error(`Failed to fetch weight and nutrition: ${res.status}`);
	}

	return WeightAndNutritionSchema.parse(await res.json());
}

/**
 * Save the weight and nutrition values for a given date.
 * @param date - The date the values apply to
 * @param patch - The values to save: omitted fields are left untouched, `null` clears them
 * @returns true if the values were updated, false otherwise
 */
export async function saveWeightAndNutrition(
	date: Date | string,
	patch: UpdateWeightAndNutritionPatch
): Promise<boolean> {
	const body = UpdateWeightAndNutritionSchema.parse(patch);
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/weight-and-nutrition/${formattedDate}`, {
		method: 'PATCH',
		mode: 'cors',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

	if (res.status === 401) {
		goto(resolve('/login'));
		return false;
	}

	return res.status === 204;
}

const WeightAndNutritionHistoryResponse = z.object({
	unprocessable_files: z.array(z.tuple([z.string(), z.string()]))
});

export type WeightAndNutritionHistoryError =
	| { type: 'partialFailure'; files: { file: string; reason: string }[] }
	| { type: 'totalFailure'; reason: string }
	| { type: 'success' };

export async function importWeightAndNutritionHistory(
	files: FileList
): Promise<WeightAndNutritionHistoryError> {
	if (files.length === 0) {
		return { type: 'success' };
	}

	const formData = new FormData();
	for (let i = 0; i < files.length; i++) {
		const file = files.item(i)!;
		formData.append(file.name, file);
	}

	const response = await fetch(`${PUBLIC_APP_URL}/api/training/weight-and-nutrition/history`, {
		method: 'POST',
		credentials: 'include',
		mode: 'cors',
		body: formData
	});

	if (response.status >= 300) {
		return {
			type: 'totalFailure',
			reason: response.status === 422 ? 'No valid date range found in data' : 'Unknown'
		};
	}

	const res = WeightAndNutritionHistoryResponse.parse(await response.json());
	if (res.unprocessable_files.length === 0) {
		return { type: 'success' };
	}

	return {
		type: 'partialFailure',
		files: res.unprocessable_files.map(([file, reason]) => ({ file, reason }))
	};
}

/**
 * Delete the weight and nutrition values for a given date.
 * @param date - The date the values apply to
 * @returns true if the values were deleted, false otherwise
 */
export async function deleteWeightAndNutrition(date: Date | string): Promise<boolean> {
	const formattedDate = dayjs(date).format('YYYY-MM-DD');

	const res = await fetch(`${PUBLIC_APP_URL}/api/training/weight-and-nutrition/${formattedDate}`, {
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
