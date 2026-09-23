import {
	type TrainingMetric,
	type TrainingMetricTemplate,
	type TrainingMetricBasePayload,
	type TrainingMetricFilters,
	getMetricGroupBy,
	getMetricFilters,
	type TrainingMetricBasePayloadFilters
} from '$lib/api/training';
import { dayjs, granularityUnits } from '$lib/duration';
import { asOption, isNone, isSome, none, some, unwrapOr, type Option } from '$lib/Options';
import type { RPEValue } from '$lib/rpe';
import type { Sport, SportCategory } from '$lib/sport';
import type { TrainingMetricGranularity, TrainingMetricGroupByClause } from '$lib/trainingMetric';
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

const fieldsActiveFilters = (fields: TrainingMetricFields): TrainingMetricBasePayloadFilters => {
	let activeFilters: TrainingMetricBasePayloadFilters = {
		bonked: null,
		rpes: null,
		sports: null,
		workout_types: null
	};

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
			workout_types: fields.filters.workoutTypes.value
		};
	}

	if (isSome(fields.filters.bonked)) {
		activeFilters = {
			...activeFilters,
			bonked: fields.filters.bonked.value
		};
	}

	if (isSome(fields.filters.rpes) && fields.filters.rpes.value.length > 0) {
		activeFilters = { ...activeFilters, rpes: fields.filters.rpes.value };
	}

	return activeFilters;
};

const convertTemplateSource = (
	template: TrainingMetricTemplate,
	group_by: Option<TrainingMetricGroupByClause>,
	filters: TrainingMetricBasePayloadFilters
): TrainingMetricBasePayload['source'] => {
	if (template.source.type === 'activity') {
		return {
			type: 'activity',
			metric: { metric: template.source.metric, group_by: unwrapOr(group_by, null), filters }
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
	const activeFilters = fieldsActiveFilters(fields);
	let payload: Omit<TrainingMetricBasePayload, 'name'> = {
		source: convertTemplateSource(fields.selectedTemplate.value, fields.groupBy, activeFilters)
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

export const matchTemplate = (
	metric: TrainingMetric,
	template: TrainingMetricTemplate
): boolean => {
	if (metric.source.type === 'activity' && template.source.type === 'activity') {
		if (metric.source.metric.metric !== template.source.metric) {
			return false;
		}
		return metric.aggregate === null ? true : metric.aggregate === template.aggregate;
	} else if (metric.source.type === 'hooperIndex' && template.source.type === 'hooperIndex') {
		return metric.source.metric === template.source.metric;
	} else if (
		metric.source.type === 'weightAndNutrition' &&
		template.source.type === 'weightAndNutrition'
	) {
		return metric.source.metric === template.source.metric;
	}
	return false;
};

const convertFilters = (filters: Option<TrainingMetricFilters>): TrainingMetricFiltersType => {
	if (isNone(filters)) {
		return {
			bonked: none(),
			rpes: none(),
			sportCategories: none(),
			sports: none(),
			workoutTypes: none()
		};
	}

	const sports: Sport[] = [];
	const categories: SportCategory[] = [];
	for (const item of filters.value.sports || []) {
		if ('Sport' in item) {
			sports.push(item.Sport);
		} else if ('SportCategory' in item) {
			categories.push(item.SportCategory);
		}
	}

	return {
		bonked: asOption(filters.value.bonked),
		rpes: asOption(filters.value.rpes) as Option<RPEValue[]>,
		sports: sports.length === 0 ? none() : some(sports),
		sportCategories: categories.length === 0 ? none() : some(categories),
		workoutTypes: asOption(filters.value.workout_types)
	};
};

export const matchMetricToFormFields = (
	metric: TrainingMetric,
	templates: TrainingMetricTemplate[]
): TrainingMetricFields => {
	const selectedTemplate = templates.find((template) => matchTemplate(metric, template));

	const filters = convertFilters(getMetricFilters(metric));

	const groupBy = getMetricGroupBy(metric);

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

export type TimeDomain = Option<{ start: string; end: string | null }>;

export const expectedBinsForDomain = (
	domain: TimeDomain,
	granularity: Option<TrainingMetricGranularity>,
	now: dayjs.Dayjs
): Option<string[]> => {
	if (isNone(domain)) {
		return none();
	}

	const granularityUnit = granularityUnits(granularity);
	let start = dayjs(domain.value.start).startOf(granularityUnit.startOf);
	const end = domain.value.end === null ? now : dayjs(domain.value.end);
	const times = [];
	while (start <= end) {
		times.push(start.format('YYYY-MM-DD'));
		start = start.add(1, granularityUnit.add);
	}

	return some(times);
};
