import { describe, expect, it } from 'vitest';

import type { TrainingMetric, TrainingMetricFilters } from '$lib/api';

import {
	bucketOffset,
	compareAnchor,
	compareBucketDomain,
	definitionLabel,
	metricPreviewPayload,
	metricDefinitionKey,
	extractBaseDefinitionFromMetric,
	periodMetricRange,
	relativeBucketLabel,
	type CompareMetricDefinition
} from './trainingMetric';
import dayjs from 'dayjs';

const metricFilters = (overrides: Partial<TrainingMetricFilters> = {}): TrainingMetricFilters => ({
	sports: [{ Sport: 'Running' }, { SportCategory: 'Cycling' }],
	workout_types: ['easy'],
	bonked: 'none',
	rpes: [5, 7],
	...overrides
});

const makeMetric = (overrides: Partial<TrainingMetric> = {}): TrainingMetric => ({
	id: 'metric-1',
	name: 'My Metric',
	source: {
		type: 'activity',
		metric: {
			metric: 'Distance',
			group_by: 'Sport',
			filters: metricFilters()
		}
	},
	unit: 'm',
	scope: { type: 'global' },
	granularity: 'Weekly',
	aggregate: 'Sum',
	show_average: { include_zeros: false },
	target: { value: 100, unit: 'km' },
	values: { no_group: { '2026-01-01': 10 } },
	summary: { average: 10 },
	...overrides
});

const makePeriod = (overrides: Partial<{ start: string; end: string | null }> = {}) => ({
	start: '2026-02-02',
	end: '2026-04-30' as string | null,
	sports: { sports: ['TrailRunning'] as 'TrailRunning'[], categories: ['Cycling'] as 'Cycling'[] },
	...overrides
});

describe('metricDefinitionKey', () => {
	it('is equal for identical definitions regardless of name or target', () => {
		const keyA = metricDefinitionKey(makeMetric());
		const keyB = metricDefinitionKey(makeMetric({ id: 'other', name: 'Renamed', target: null }));

		expect(keyA).toBe(keyB);
	});

	it('differs when a definition parameter differs', () => {
		expect(metricDefinitionKey(makeMetric())).not.toBe(
			metricDefinitionKey(makeMetric({ granularity: 'Daily' }))
		);
		// The definition key includes the whole source, filters included
		expect(metricDefinitionKey(makeMetric())).not.toBe(
			metricDefinitionKey(
				makeMetric({
					source: {
						type: 'activity',
						metric: {
							metric: 'Distance',
							group_by: 'Sport',
							filters: metricFilters({ sports: null })
						}
					}
				})
			)
		);
	});
});

describe('metricToPreviewBase', () => {
	it('carries source with filters, window, summary and target', () => {
		const base = extractBaseDefinitionFromMetric(makeMetric());

		expect(base.source).toStrictEqual({
			type: 'activity',
			metric: {
				metric: 'Distance',
				group_by: 'Sport',
				filters: {
					sports: [{ Sport: 'Running' }, { SportCategory: 'Cycling' }],
					workout_types: ['easy'],
					bonked: 'none',
					rpes: [5, 7]
				}
			}
		});
		expect(base.window).toEqual({
			granularity: 'Weekly',
			aggregate: 'Sum'
		});
		expect(base.summary).toEqual({ average: { include_zeros: false } });
		expect(base.target).toEqual({ value: 100, unit: 'km' });
	});

	it('keeps unset sports filters as-is so each period applies its own sports', () => {
		const base = extractBaseDefinitionFromMetric(
			makeMetric({
				source: {
					type: 'activity',
					metric: {
						metric: 'Distance',
						group_by: 'Sport',
						filters: metricFilters({ sports: null })
					}
				}
			})
		);

		if (base.source.type !== 'activity') {
			throw new Error('expected an activity source');
		}
		expect(base.source.metric.filters.sports).toBeNull();
	});

	it('omits window, summary and target when unset', () => {
		const base = extractBaseDefinitionFromMetric(
			makeMetric({
				granularity: null,
				aggregate: null,
				show_average: null,
				target: null
			})
		);

		expect(base.window).toBeUndefined();
		expect(base.summary).toBeUndefined();
		expect(base.target).toBeUndefined();
	});
});

describe('compareMetricPreviewPayload', () => {
	it('computes each period range with an inclusive end', () => {
		const definition: CompareMetricDefinition = {
			key: 'default:weekly-distance',
			label: 'Weekly distance',
			source: 'default',
			base: {
				source: {
					type: 'activity',
					metric: {
						metric: 'Distance',
						group_by: null,
						filters: metricFilters({ sports: null, workout_types: null, bonked: null, rpes: null })
					}
				},
				window: { granularity: 'Weekly', aggregate: 'Sum' }
			}
		};

		const payload = metricPreviewPayload(definition, makePeriod());

		expect(payload.source).toStrictEqual(definition.base.source);
		expect(payload.start).toBe('2026-02-02');
		expect(payload.end).toBe('2026-05-01');
	});

	it('keeps the definition filters unchanged in the payload', () => {
		const definition: CompareMetricDefinition = {
			key: 'key',
			label: null,
			source: 'first',
			base: {
				source: {
					type: 'activity',
					metric: {
						metric: 'Distance',
						group_by: null,
						filters: metricFilters({ sports: [{ Sport: 'Cycling' }] })
					}
				},
				window: { granularity: 'Weekly', aggregate: 'Sum' }
			}
		};

		const payload = metricPreviewPayload(definition, makePeriod());

		expect(payload.source).toStrictEqual(definition.base.source);
		if (payload.source.type !== 'activity') {
			throw new Error('expected an activity source');
		}
		expect(payload.source.metric.filters.sports).toEqual([{ Sport: 'Cycling' }]);
	});

	it('uses tomorrow for an ongoing period', () => {
		const now = () => dayjs('2026-09-08T15:00:00+02:00');

		const definition: CompareMetricDefinition = {
			key: 'key',
			label: null,
			source: 'default',
			base: {
				source: {
					type: 'activity',
					metric: {
						metric: 'Distance',
						group_by: null,
						filters: metricFilters({ sports: null, workout_types: null, bonked: null, rpes: null })
					}
				},
				window: { granularity: 'Weekly', aggregate: 'Sum' }
			}
		};

		const payload = metricPreviewPayload(definition, makePeriod({ end: null }), now);

		expect(payload.end).toBe('2026-09-09');
	});
});

describe('periodMetricRange', () => {
	it('extends the end by one day', () => {
		expect(periodMetricRange({ start: '2026-02-02', end: '2026-04-30' })).toEqual({
			start: '2026-02-02',
			end: '2026-05-01'
		});
	});
});

describe('compareAnchor', () => {
	it('returns the period start when aligning by start', () => {
		expect(compareAnchor(makePeriod(), 'start')).toBe('2026-02-02');
	});

	it('returns the period end when aligning by end', () => {
		expect(compareAnchor(makePeriod(), 'end')).toBe('2026-04-30');
	});

	it('falls back to the start when aligning an ongoing period by end', () => {
		expect(compareAnchor(makePeriod({ end: null }), 'end')).toBe('2026-02-02');
	});
});

describe('bucketOffset', () => {
	it('counts weeks between the bucket and the anchor', () => {
		expect(bucketOffset('2026-01-05', '2026-01-05', 'Weekly')).toBe(0);
		expect(bucketOffset('2026-01-19', '2026-01-05', 'Weekly')).toBe(2);
		expect(bucketOffset('2025-12-29', '2026-01-05', 'Weekly')).toBe(-1);
	});

	it('counts days between the bucket and the anchor', () => {
		expect(bucketOffset('2026-01-05', '2026-01-05', 'Daily')).toBe(0);
		expect(bucketOffset('2026-01-08', '2026-01-05', 'Daily')).toBe(3);
	});

	it('counts months between the bucket and the anchor', () => {
		expect(bucketOffset('2026-03-01', '2026-01-05', 'Monthly')).toBe(2);
	});

	it('is normalized within the granule', () => {
		// Mid-week anchors still align whole weeks (Jan 7 → week of Jan 5)
		expect(bucketOffset('2026-01-19', '2026-01-07', 'Weekly')).toBe(2);
	});
});

describe('relativeBucketLabel', () => {
	it('numbers buckets from the first offset of the shared domain', () => {
		expect(relativeBucketLabel(0, 0, 'Weekly')).toBe('Week 1');
		expect(relativeBucketLabel(2, 0, 'Weekly')).toBe('Week 3');
		expect(relativeBucketLabel(4, 0, 'Daily')).toBe('Day 5');
		expect(relativeBucketLabel(1, 0, 'Monthly')).toBe('Month 2');
	});

	it('numbers from the domain start whatever the offset sign', () => {
		expect(relativeBucketLabel(-5, -8, 'Weekly')).toBe('Week 4');
		expect(relativeBucketLabel(0, -3, 'Monthly')).toBe('Month 4');
	});
});

describe('compareBucketDomain', () => {
	it('unions and sorts the bucket offsets of both metrics', () => {
		const first = makeMetric({
			values: { no_group: { '2026-02-02': 1, '2026-02-09': 2 } }
		});
		const second = makeMetric({
			values: { no_group: { '2026-02-09': 3, '2026-02-16': 4 } }
		});

		expect(compareBucketDomain([first, second], ['2026-02-02', '2026-02-02'])).toEqual([0, 1, 2]);
	});

	it('ignores missing metrics', () => {
		const metric = makeMetric({
			values: { no_group: { '2026-02-02': 1 } }
		});

		expect(compareBucketDomain([undefined, metric], [undefined, '2026-02-02'])).toEqual([0]);
		expect(compareBucketDomain([], [])).toEqual([]);
	});
});

describe('compareDefinitionLabel', () => {
	it('returns the label when set', () => {
		const definition: CompareMetricDefinition = {
			key: 'key',
			label: 'Weekly distance',
			source: 'default',
			base: {
				source: {
					type: 'activity',
					metric: {
						metric: 'Distance',
						group_by: null,
						filters: metricFilters({ sports: null, workout_types: null, bonked: null, rpes: null })
					}
				},
				window: { granularity: 'Weekly', aggregate: 'Sum' }
			}
		};

		expect(definitionLabel(definition)).toBe('Weekly distance');
	});

	it('composes the label from the definition otherwise', () => {
		expect(
			definitionLabel({
				key: 'key',
				label: null,
				source: 'first',
				base: {
					source: {
						type: 'activity',
						metric: {
							metric: 'Distance',
							group_by: null,
							filters: metricFilters({
								sports: null,
								workout_types: null,
								bonked: null,
								rpes: null
							})
						}
					},
					window: { granularity: 'Weekly', aggregate: 'Sum' }
				}
			})
		).toBe('weekly total distance');

		expect(
			definitionLabel({
				key: 'key',
				label: null,
				source: 'first',
				base: {
					source: {
						type: 'activity',
						metric: {
							metric: 'ActiveDuration',
							group_by: null,
							filters: metricFilters({
								sports: null,
								workout_types: null,
								bonked: null,
								rpes: null
							})
						}
					},
					window: { granularity: 'Daily', aggregate: 'Max' }
				}
			})
		).toBe('daily maximum activeduration');
	});
});
