import type {
	TrainingMetric,
	TrainingMetricTemplate,
	TrainingMetricBasePayload
} from '$lib/api/training';
import { bonkStatusToAPI } from '$lib/nutrition';
import { asOption, isNone, isSome, none, some, type Option } from '$lib/Options';
import type { Sport, SportCategory } from '$lib/sport';
import type { TrainingMetricGranularity, TrainingMetricGroupByClause } from '$lib/trainingMetric';
import { workoutTypeToAPI } from '$lib/workout-type';
import type { TrainingMetricFiltersType } from '../TrainingMetricFilters.svelte';

export interface TrainingMetricFields {
	name: string;
	selectedTemplate: Option<TrainingMetricTemplate>;
	granularity: Option<TrainingMetricGranularity>;
	groupBy: Option<TrainingMetricGroupByClause>;
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

export const fieldsAsPayload = (
	fields: TrainingMetricFields
): Option<TrainingMetricBasePayload> => {
	if (isNone(fields.selectedTemplate)) {
		return none();
	}
	let payload: Omit<TrainingMetricBasePayload, 'name'> = {
		metric: fields.selectedTemplate.value.metric
	};

	// Optional window
	if (isSome(fields.granularity)) {
		let window: TrainingMetricBasePayload['window'] = {
			granularity: fields.granularity.value,
			aggregate: fields.selectedTemplate.value.aggregate
		};

		if (isSome(fields.groupBy)) {
			window = { ...window, group_by: fields.groupBy.value };
		}

		payload = { ...payload, window };
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
	metric: TrainingMetric,
	templates: TrainingMetricTemplate[]
): TrainingMetricFields => {
	const selectedTemplate = templates.find(
		(template) =>
			metric.metric === template.metric &&
			(metric.aggregate === null ? true : metric.aggregate === template.aggregate)
	);

	const filters = {
		sports: metric.sports === null ? none() : some(metric.sports.sports),
		sportCategories: metric.sports === null ? none() : some(metric.sports.categories),
		bonked: asOption(metric.bonked),
		rpes: asOption(metric.rpes),
		workoutTypes: asOption(metric.workout_types)
	} as TrainingMetricFiltersType;

	return {
		name: metric.name || '',
		selectedTemplate: selectedTemplate === undefined ? none() : some(selectedTemplate),
		granularity: asOption(metric.granularity),
		groupBy: asOption(metric.group_by),
		showAverage: metric.show_average !== null,
		filters
	};
};
