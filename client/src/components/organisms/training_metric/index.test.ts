import { describe, expect, it } from 'vitest';

import type { TrainingMetric, TrainingMetricTemplate } from '$lib/api/training';
import { isNone, isSome, some } from '$lib/Options';

import { matchMetricToFormFields } from './index';

const makeTemplate = (overrides: Partial<TrainingMetricTemplate> = {}): TrainingMetricTemplate => ({
	display_name: 'Total Duration',
	metric: 'ActiveDuration',
	aggregate: 'Sum',
	unit: 's',
	category: 'Duration',
	...overrides
});

const makeMetric = (overrides: Partial<TrainingMetric> = {}): TrainingMetric => ({
	id: 'metric-1',
	name: 'My Metric',
	metric: 'ActiveDuration',
	unit: 's',
	scope: { type: 'global' },
	granularity: 'Weekly',
	aggregate: 'Sum',
	group_by: 'Sport',
	sports: {
		sports: ['Running'],
		categories: ['Running']
	},
	workout_types: ['easy'],
	bonked: 'none',
	rpes: [5, 7],
	show_average: { include_zeros: false },
	values: { no_group: { '2026-01-01': 10 } },
	summary: { total: 10 },
	...overrides
});

describe('matchMetricToFormFields', () => {
	it('maps a fully populated metric into form fields', () => {
		const template = makeTemplate();
		const metric = makeMetric();

		const result = matchMetricToFormFields(metric, [template]);

		expect(result.name).toBe('My Metric');
		expect(result.showAverage).toBe(true);
		expect(result.granularity).toEqual(some('Weekly'));
		expect(result.groupBy).toEqual(some('Sport'));

		expect(result.filters.sports).toEqual(some(['Running']));
		expect(result.filters.sportCategories).toEqual(some(['Running']));
		expect(result.filters.workoutTypes).toEqual(some(['easy']));
		expect(result.filters.bonked).toEqual(some('none'));
		expect(result.filters.rpes).toEqual(some([5, 7]));

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(template);
		}
	});

	it('returns none/empty defaults for nullable metric fields', () => {
		const metric = makeMetric({
			name: null,
			granularity: null,
			group_by: null,
			sports: null,
			workout_types: null,
			bonked: null,
			rpes: null,
			show_average: null
		});

		const result = matchMetricToFormFields(metric, []);

		expect(result.name).toBe('');
		expect(result.showAverage).toBe(false);
		expect(isNone(result.granularity)).toBe(true);
		expect(isNone(result.groupBy)).toBe(true);
		expect(isNone(result.filters.sports)).toBe(true);
		expect(isNone(result.filters.sportCategories)).toBe(true);
		expect(isNone(result.filters.workoutTypes)).toBe(true);
		expect(isNone(result.filters.bonked)).toBe(true);
		expect(isNone(result.filters.rpes)).toBe(true);
		expect(isNone(result.selectedTemplate)).toBe(true);
	});

	it('matches template by metric and aggregate when aggregate is set', () => {
		const matchingTemplate = makeTemplate({ aggregate: 'Average' });
		const wrongAggregateTemplate = makeTemplate({ aggregate: 'Sum' });
		const metric = makeMetric({ aggregate: 'Average' });

		const result = matchMetricToFormFields(metric, [wrongAggregateTemplate, matchingTemplate]);

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(matchingTemplate);
		}
	});

	it('matches by metric only when metric aggregate is null', () => {
		const firstMetricTemplate = makeTemplate({ aggregate: 'Min' });
		const secondMetricTemplate = makeTemplate({ aggregate: 'Max' });
		const metric = makeMetric({ aggregate: null });

		const result = matchMetricToFormFields(metric, [firstMetricTemplate, secondMetricTemplate]);

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(firstMetricTemplate);
		}
	});
});
