import {
	type TrainingMetric,
	type TrainingMetricTemplate,
	type TrainingMetricBasePayload,
	metricGroupBy
} from '$lib/api/training';
import { bonkStatusToAPI } from '$lib/nutrition';
import { asOption, isNone, isSome, none, some, unwrapOr, type Option } from '$lib/Options';
import type { Sport, SportCategory } from '$lib/sport';
import type { TrainingMetricGranularity, TrainingMetricGroupByClause } from '$lib/trainingMetric';
import { workoutTypeToAPI } from '$lib/workout-type';
import type { TrainingMetricFiltersType } from './internal/TrainingMetricFilters.svelte';

export interface TrainingMetricFields {
	name: string;
	selectedTemplate: Option<TrainingMetricTemplate>;
	granularity: Option<TrainingMetricGranularity>;
	groupBy: Option<TrainingMetricGroupByClause>;
	filters: TrainingMetricFiltersType;
	showAverage: boolean;
	target: Option<number>;
}

export const emptyTrainingMetricFields = (): TrainingMetricFields => {
	return {
		name: '',
		selectedTemplate: none(),
		granularity: none(),
		groupBy: none(),
		filters: {
			sports: none(),
			sportCategories: none(),
			rpes: none(),
			workoutTypes: none(),
			bonked: none()
		},
		showAverage: false,
		target: none()
	};
};

export const fieldsAreEmpty = (fields: TrainingMetricFields): boolean => {
	return fields.name === '' || isNone(fields.selectedTemplate);
};

export type Scope = { kind: 'global' } | { kind: 'period'; periodId: string };

const fieldsActiveFilters = (fields: TrainingMetricFields) => {
	let activeFilters: object = {};

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

const convertTemplateSource = (
	template: TrainingMetricTemplate,
	group_by: Option<TrainingMetricGroupByClause>
): TrainingMetricBasePayload['source'] => {
	if (template.source.type === 'activity') {
		return {
			type: 'activity',
			metric: { metric: template.source.metric, group_by: unwrapOr(group_by, null) }
		};
	} else {
		return { type: template.source.type, metric: template.source.metric };
	}
};

export const fieldsAsPayload = (
	fields: TrainingMetricFields
): Option<TrainingMetricBasePayload> => {
	if (isNone(fields.selectedTemplate)) {
		return none();
	}
	let payload: Omit<TrainingMetricBasePayload, 'name'> = {
		source: convertTemplateSource(fields.selectedTemplate.value, fields.groupBy)
	};

	// Optional window
	if (isSome(fields.granularity)) {
		let window: TrainingMetricBasePayload['window'] = {
			granularity: fields.granularity.value,
			aggregate: fields.selectedTemplate.value.aggregate
		};

		if (isSome(fields.groupBy)) {
			window = { ...window };
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

	// Optional target: the unit is forced to the selected template's unit
	if (isSome(fields.target)) {
		payload = {
			...payload,
			target: {
				value: fields.target.value,
				unit: fields.selectedTemplate.value.unit
			}
		};
	}

	return some(payload);
};

const matchTemplate = (metric: TrainingMetric, template: TrainingMetricTemplate): boolean => {
	if (metric.source.type === 'activity' && template.source.type === 'activity') {
		if (metric.source.metric.metric !== template.source.metric) {
			return false;
		}
	} else if (metric.source.type === 'hooperIndex' && template.source.type === 'hooperIndex') {
		if (metric.source.metric !== template.source.metric) {
			return false;
		}
	} else if (
		metric.source.type === 'weightAndNutrition' &&
		template.source.type === 'weightAndNutrition'
	) {
		if (metric.source.metric !== template.source.metric) {
			return false;
		}
	}
	return metric.aggregate === null ? true : metric.aggregate === template.aggregate;
};

export const matchMetricToFormFields = (
	metric: TrainingMetric,
	templates: TrainingMetricTemplate[]
): TrainingMetricFields => {
	const selectedTemplate = templates.find((template) => matchTemplate(metric, template));

	const filters = {
		sports: metric.sports === null ? none() : some(metric.sports.sports),
		sportCategories: metric.sports === null ? none() : some(metric.sports.categories),
		bonked: asOption(metric.bonked),
		rpes: asOption(metric.rpes),
		workoutTypes: asOption(metric.workout_types)
	} as TrainingMetricFiltersType;

	const groupBy = metricGroupBy(metric);

	return {
		name: metric.name || '',
		selectedTemplate: selectedTemplate === undefined ? none() : some(selectedTemplate),
		granularity: asOption(metric.granularity),
		groupBy,
		showAverage: metric.show_average !== null,
		target: asOption(metric.target?.value ?? null),
		filters
	};
};
