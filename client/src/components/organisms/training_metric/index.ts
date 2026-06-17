import type {  MetricsListItemGrouped, MetricTemplate } from '$lib/api/training';
import { bonkStatusToAPI } from '$lib/nutrition';
import { isNone, isSome, none, some, type Option } from '$lib/Options';
import type { Sport, SportCategory } from '$lib/sport';
import { workoutTypeToAPI } from '$lib/workout-type';
import type { TrainingMetricFiltersType } from '../TrainingMetricFilters.svelte';

export const granularityValues = ['None', 'Daily', 'Weekly', 'Monthly'] as const;
export type Granularity = (typeof granularityValues)[number];

export const gropuByValues = [
	'None',
	'Sport',
	'SportCategory',
	'WorkoutType',
	'RpeRange',
	'Bonked'
] as const;
export type GroupBy = (typeof gropuByValues)[number];

export interface TrainingMetricFields {
	name: string;
	selectedTemplate: Option<MetricTemplate>;
	granularity: Granularity;
	groupBy: GroupBy;
	filters: TrainingMetricFiltersType;
	showAverage: boolean;
}

export type Scope = { kind: 'global' } | { kind: 'period'; periodId: string };

const fieldsActiveFilters = (fields: TrainingMetricFields) => {
	let activeFilters: Object = {};

	if (isSome(fields.filters.sports) && isSome(fields.filters.sportCategories)) {
		const sportFilter = fields.filters.sports.value.map((sport) => ({
			Sport: sport
		}));
		const sportCategoriesFilter = fields.filters.sportCategories.value.map((category) => ({
			SportCategory: category
		}));
		const sportFilters: ({ Sport: Sport } | { SportCategory: SportCategory })[] = [
			...sportFilter,
			...sportCategoriesFilter
		];
		if (sportFilters.length > 0) {
			activeFilters = { ...activeFilters, sports: sportFilters };
		}
	}

	if (isSome(fields.filters.workoutTypes) && fields.filters.workoutTypes.value.length > 0) {
		activeFilters = {
			...activeFilters,
			workout_types: fields.filters.workoutTypes.value.map(workoutTypeToAPI)
		};
	}

	if (isSome(fields.filters.bonked)) {
		activeFilters = {
			...activeFilters,
			bonked: bonkStatusToAPI(fields.filters.bonked.value)
		};
	}

	if (isSome(fields.filters.rpes) && fields.filters.rpes.value.length > 0) {
		activeFilters = { ...activeFilters, rpes: fields.filters.rpes.value };
	}

	return activeFilters;
};

export const fieldsAsPayload = (fields: TrainingMetricFields): Option<Object> => {
	if (isNone(fields.selectedTemplate)) {
		return none();
	}
	let payload: Object = {
		metric: fields.selectedTemplate.value.metric
	};

	// Optional window
	if (fields.granularity !== 'None') {
		let window: {} = {
			granularity: fields.granularity,
			aggregate: fields.selectedTemplate.value.aggregate
		};

		if (fields.groupBy !== 'None') {
			window = { ...window, group_by: fields.groupBy };
		}

		payload = { window, ...payload };
	}

	// Optional filters
	const activeFilters = fieldsActiveFilters(fields);
	if (Object.keys(activeFilters).length > 0) {
		payload = { ...payload, filters: activeFilters };
	}

	// Optional summary
	if (fields.showAverage) {
		payload = { ...payload, summary: { average: { include_zeros: false } } };
	}

	return some(payload);
};

export const matchMetricToFormFields = (
	metric: MetricsListItemGrouped,
	templates: MetricTemplate[]
): TrainingMetricFields => {
	const selectedTemplate = templates.find((template) =>
		metric.metric === template.metric && metric.aggregate === null
			? true
			: metric.aggregate === template.aggregate
	);

	const filters = {
		sports: metric.sports === null ? none() : some(metric.sports?.sports),
		sportCategories: metric.sports === null ? none() : some(metric.sports?.categories),
		bonked: metric.bonked === null ? none() : some(metric.bonked),
		rpes: metric.rpes === null ? none() : some(metric.rpes),
		workoutTypes: metric.workout_types === null ? none() : some(metric.workout_types)
	} as TrainingMetricFiltersType;

	return {
		name: metric.name || '',
		selectedTemplate: selectedTemplate === undefined ? none() : some(selectedTemplate),
		granularity: metric.granularity === null ? 'None' : (metric.granularity as Granularity),
		groupBy: metric.group_by === null ? 'None' : metric.group_by,
		showAverage: metric.show_average !== null,
		filters
	};
};
