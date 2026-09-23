import { describe, expect, it } from 'vitest';

import type { TrainingMetric, TrainingMetricTemplate } from '$lib/api/training';
import { isNone, isSome, none, some } from '$lib/Options';

import {
	expectedBinsForDomain,
	fieldsAsPayload,
	matchMetricToFormFields,
	matchTemplate,
	type TrainingMetricFields
} from './index';
import { dayjs } from '$lib/duration';

const activityMetricSource = (metric: string): TrainingMetric['source'] => ({
	type: 'activity',
	metric: {
		metric,
		group_by: 'Sport',
		filters: {
			sports: [{ Sport: 'Running' }, { SportCategory: 'Running' }],
			workout_types: ['easy'],
			bonked: 'none',
			rpes: [5, 7]
		}
	}
});

const makeTemplate = ({
	source,
	aggregate,
	...overrides
}: Pick<TrainingMetricTemplate, 'source' | 'aggregate'> &
	Partial<TrainingMetricTemplate>): TrainingMetricTemplate => ({
	display_name: 'Total Duration',
	unit: 's',
	category: 'Duration',
	...overrides,
	source,
	aggregate
});

const makeMetric = ({
	source,
	aggregate,
	...overrides
}: Pick<TrainingMetric, 'source' | 'aggregate'> & Partial<TrainingMetric>): TrainingMetric => ({
	id: 'metric-1',
	name: 'My Metric',
	unit: 's',
	scope: { type: 'global' },
	granularity: 'Weekly',
	show_average: { include_zeros: false },
	target: { value: 100, unit: 'km' },
	values: { no_group: { '2026-01-01': 10 } },
	summary: { total: 10 },
	...overrides,
	source,
	aggregate
});

describe('matchMetricToFormFields', () => {
	it('maps a fully populated metric into form fields', () => {
		const template = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Sum'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Sum'
		});

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
		expect(result.target).toEqual(some(100));

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(template);
		}
	});

	it('returns none/empty defaults for nullable metric fields', () => {
		const metric = makeMetric({
			name: null,
			source: {
				type: 'activity',
				metric: {
					metric: 'ActiveDuration',
					group_by: null,
					filters: {
						bonked: null,
						rpes: null,
						sports: null,
						workout_types: null
					}
				}
			},
			aggregate: 'Sum',
			granularity: null,
			show_average: null,
			target: null
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
		expect(isNone(result.target)).toBe(true);
	});

	it('matches template by metric and aggregate when aggregate is set', () => {
		const matchingTemplate = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Average'
		});
		const wrongAggregateTemplate = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Sum'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Average'
		});

		const result = matchMetricToFormFields(metric, [wrongAggregateTemplate, matchingTemplate]);

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(matchingTemplate);
		}
	});

	it('matches by metric only when metric aggregate is null', () => {
		const firstMetricTemplate = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Min'
		});
		const secondMetricTemplate = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Max'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: null
		});

		const result = matchMetricToFormFields(metric, [firstMetricTemplate, secondMetricTemplate]);

		expect(isSome(result.selectedTemplate)).toBe(true);
		if (isSome(result.selectedTemplate)) {
			expect(result.selectedTemplate.value).toEqual(firstMetricTemplate);
		}
	});
});

describe('matchTemplate', () => {
	it('matches activity templates with the same metric and aggregate', () => {
		const template = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Sum'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Sum'
		});

		expect(matchTemplate(metric, template)).toBe(true);
	});

	it('matches any activity template when the metric aggregate is null', () => {
		const template = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Max'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: null
		});

		expect(matchTemplate(metric, template)).toBe(true);
	});

	it('does not match activity templates with a different aggregate', () => {
		const template = makeTemplate({
			source: { type: 'activity', metric: 'ActiveDuration' },
			aggregate: 'Sum'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Average'
		});

		expect(matchTemplate(metric, template)).toBe(false);
	});

	it('does not match activity templates with a different metric', () => {
		const template = makeTemplate({
			source: { type: 'activity', metric: 'Distance' },
			aggregate: 'Sum'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Sum'
		});

		expect(matchTemplate(metric, template)).toBe(false);
	});

	it('matches hooperIndex templates with the same metric', () => {
		const template = makeTemplate({
			source: { type: 'hooperIndex', metric: 'Fatigue' },
			aggregate: 'Average',
			unit: 'index',
			category: 'Feedback'
		});
		const metric = makeMetric({
			source: { type: 'hooperIndex', metric: 'Fatigue' },
			aggregate: 'Average'
		});

		expect(matchTemplate(metric, template)).toBe(true);
	});

	it('does not match hooperIndex templates with a different metric', () => {
		const template = makeTemplate({
			source: { type: 'hooperIndex', metric: 'Fatigue' },
			aggregate: 'Average',
			unit: 'index',
			category: 'Feedback'
		});
		const metric = makeMetric({
			source: { type: 'hooperIndex', metric: 'Stress' },
			aggregate: 'Average'
		});

		expect(matchTemplate(metric, template)).toBe(false);
	});

	it('matches weightAndNutrition templates with the same metric', () => {
		const template = makeTemplate({
			source: { type: 'weightAndNutrition', metric: 'Weight' },
			aggregate: 'Average',
			unit: 'kg',
			category: 'Weight'
		});
		const metric = makeMetric({
			source: { type: 'weightAndNutrition', metric: 'Weight' },
			aggregate: 'Average'
		});

		expect(matchTemplate(metric, template)).toBe(true);
	});

	it('does not match weightAndNutrition templates with a different metric', () => {
		const template = makeTemplate({
			source: { type: 'weightAndNutrition', metric: 'Weight' },
			aggregate: 'Average',
			unit: 'kg',
			category: 'Weight'
		});
		const metric = makeMetric({
			source: { type: 'weightAndNutrition', metric: 'Calories' },
			aggregate: 'Average'
		});

		expect(matchTemplate(metric, template)).toBe(false);
	});

	it('does not match when the source types differ', () => {
		const hooperTemplate = makeTemplate({
			source: { type: 'hooperIndex', metric: 'ActiveDuration' },
			aggregate: 'Sum',
			unit: 'index',
			category: 'Feedback'
		});
		const metric = makeMetric({
			source: activityMetricSource('ActiveDuration'),
			aggregate: 'Sum'
		});

		expect(matchTemplate(metric, hooperTemplate)).toBe(false);
	});
});

describe('fieldsAsPayload', () => {
	const makeFields = (target: number | null): TrainingMetricFields => ({
		name: 'Metric',
		selectedTemplate: some(
			makeTemplate({
				source: { type: 'activity', metric: 'ActiveDuration' },
				aggregate: 'Sum'
			})
		),
		granularity: none(),
		groupBy: none(),
		filters: {
			sports: none(),
			sportCategories: none(),
			rpes: none(),
			bonked: none(),
			workoutTypes: none()
		},
		showAverage: false,
		target: target === null ? none() : some(target)
	});

	it('includes target in the payload when set, using the template unit', () => {
		const payload = fieldsAsPayload(makeFields(100));

		expect(isSome(payload)).toBe(true);
		if (isSome(payload)) {
			expect(payload.value.target).toEqual({ value: 100, unit: 's' });
		}
	});

	it('uses the selected template unit for the target', () => {
		const fields: TrainingMetricFields = {
			...makeFields(50),
			selectedTemplate: some(
				makeTemplate({
					source: { type: 'activity', metric: 'ActiveDuration' },
					aggregate: 'Sum',
					unit: 'km'
				})
			)
		};

		const payload = fieldsAsPayload(fields);

		expect(isSome(payload)).toBe(true);
		if (isSome(payload)) {
			expect(payload.value.target).toEqual({ value: 50, unit: 'km' });
		}
	});

	it('omits target from the payload when not set', () => {
		const payload = fieldsAsPayload(makeFields(null));

		expect(isSome(payload)).toBe(true);
		if (isSome(payload)) {
			expect('target' in payload.value).toBe(false);
		}
	});
});

describe('expectedBinsForDomain', () => {
	it('returns none for a none domain', () => {
		const now = dayjs('2026-01-04');
		expect(isNone(expectedBinsForDomain(none(), some('Daily'), now))).toBe(true);
	});

	it('returns a bin per day for daily granularity', () => {
		const now = dayjs('2026-01-04');
		const bins = expectedBinsForDomain(
			some({ start: '2026-01-01', end: '2026-01-04' }),
			some('Daily'),
			now
		);

		expect(isSome(bins) && bins.value).toEqual([
			'2026-01-01',
			'2026-01-02',
			'2026-01-03',
			'2026-01-04'
		]);
	});

	it('snaps the start to the beginning of the week for weekly granularity', () => {
		const now = dayjs('2026-01-04');
		const bins = expectedBinsForDomain(
			some({ start: '2026-09-23', end: '2026-10-02' }),
			some('Weekly'),
			now
		);

		expect(isSome(bins) && bins.value).toEqual(['2026-09-21', '2026-09-28']);
	});

	it('snaps the start to the beginning of the month for monthly granularity', () => {
		const now = dayjs('2026-01-04');
		const bins = expectedBinsForDomain(
			some({ start: '2026-01-15', end: '2026-03-31' }),
			some('Monthly'),
			now
		);

		expect(isSome(bins) && bins.value).toEqual(['2026-01-01', '2026-02-01', '2026-03-01']);
	});

	it('defaults to daily bins when the granularity is none', () => {
		const now = dayjs('2026-01-04');
		const bins = expectedBinsForDomain(
			some({ start: '2026-02-01', end: '2026-02-03' }),
			none(),
			now
		);

		expect(isSome(bins) && bins.value).toEqual(['2026-02-01', '2026-02-02', '2026-02-03']);
	});

	it('uses now as the end when the domain end is null', () => {
		const now = dayjs('2026-01-02');

		const bins = expectedBinsForDomain(
			some({ start: '2026-01-01', end: null }),
			some('Daily'),
			now
		);

		expect(isSome(bins) && bins.value).toEqual(['2026-01-01', '2026-01-02']);
	});

	it('returns no bins when the domain is empty (start > end)', () => {
		const now = dayjs('2026-01-02');
		const bins = expectedBinsForDomain(
			some({ start: '2026-01-06', end: '2026-01-05' }),
			some('Daily'),
			now
		);

		expect(isSome(bins) && bins.value).toEqual([]);
	});
});
