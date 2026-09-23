import { afterEach, describe, expect, it, vi } from 'vitest';

import { isNone, isSome, none, some, unwrap } from '$lib/Options';
import type { TrainingMetric, TrainingPeriodDetails } from '$lib/api';
import type { TrainingMetricGranularity } from '$lib/trainingMetric';

import {
	computeExtendedTimeDomain,
	computeMissingNumberOfBins,
	numberOfDistinctBins
} from './CompareMetricEntry';

const makeMetric = (granularity: TrainingMetricGranularity | null = 'Daily'): TrainingMetric => ({
	id: 'metric-1',
	name: 'Distance',
	source: { type: 'hooperIndex', metric: 'Distance' },
	unit: 'm',
	scope: { type: 'global' },
	granularity,
	aggregate: 'Sum',
	show_average: null,
	target: null,
	values: {},
	summary: {}
});

const makePeriod = (overrides: Partial<TrainingPeriodDetails> = {}): TrainingPeriodDetails => ({
	id: 'period-1',
	start: '2026-02-01',
	end: '2026-04-30',
	name: 'Base block',
	sports: { sports: [], categories: [] },
	note: null,
	activities: [],
	...overrides
});

afterEach(() => {
	vi.useRealTimers();
});

describe('numberOfDistinctBins', () => {
	it('returns a bin per day for daily granularity', () => {
		// Feb 2026 has 28 days, March 31, April 30
		expect(numberOfDistinctBins('2026-02-01', '2026-04-30', 'Daily')).toBe(89);
	});

	it('snaps the start to the beginning of the week for weekly granularity', () => {
		// 2026-02-01 is a Sunday, so the first bin is the week of 2026-01-26
		expect(numberOfDistinctBins('2026-02-01', '2026-04-30', 'Weekly')).toBe(14);
	});

	it('snaps the start to the beginning of the month for monthly granularity', () => {
		expect(numberOfDistinctBins('2026-02-01', '2026-04-30', 'Monthly')).toBe(3);
	});

	it('defaults to daily bins when the granularity is null', () => {
		expect(numberOfDistinctBins('2026-02-01', '2026-02-03', null)).toBe(3);
	});

	it('uses now as the end when the period end is null', () => {
		vi.useFakeTimers({ now: new Date('2026-01-04T12:00:00') });

		expect(numberOfDistinctBins('2026-01-01', null, 'Daily')).toBe(4);
	});
});

describe('computeMissingNumberOfBins', () => {
	it('returns the missing bins for the shorter period', () => {
		const first = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-02-28' })
		};
		const second = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-03-15' })
		};

		const result = computeMissingNumberOfBins(first, second);

		expect(isSome(result.first) && result.first.value).toBe(15);
		expect(isNone(result.second)).toBe(true);
	});

	it('returns the missing bins for the second period when it is shorter', () => {
		const first = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-03-15' })
		};
		const second = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-02-28' })
		};

		const result = computeMissingNumberOfBins(first, second);

		expect(isNone(result.first)).toBe(true);
		expect(isSome(result.second) && result.second.value).toBe(15);
	});

	it('accounts for the metric granularities, not just the periods', () => {
		const first = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-02-28' })
		};
		const second = {
			metric: makeMetric('Monthly'),
			period: makePeriod({ start: '2026-02-01', end: '2026-02-28' })
		};

		const result = computeMissingNumberOfBins(first, second);

		expect(isNone(result.first)).toBe(true);
		expect(isSome(result.second) && result.second.value).toBe(27);
	});

	it('returns none for both when the bin counts match', () => {
		const first = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-02-01', end: '2026-02-28' })
		};
		const second = {
			metric: makeMetric('Daily'),
			period: makePeriod({ start: '2026-03-01', end: '2026-03-28' })
		};

		const result = computeMissingNumberOfBins(first, second);

		expect(isNone(result.first)).toBe(true);
		expect(isNone(result.second)).toBe(true);
	});
});

describe('computeExtendedTimeDomain', () => {
	it('returns the period unchanged when there is no bins delta', () => {
		const period = makePeriod();

		const domain = computeExtendedTimeDomain(period, none(), none(), 'start');

		expect(unwrap(domain)).toEqual({ start: '2026-02-01', end: '2026-04-30' });
	});

	it('extends the end when aligning by start', () => {
		const period = makePeriod();

		const domain = computeExtendedTimeDomain(period, some(2), some('Weekly'), 'start');

		expect(unwrap(domain)).toEqual({ start: '2026-02-01', end: '2026-05-14' });
	});

	it('extends the start (backwards) when aligning by end', () => {
		const period = makePeriod();

		const domain = computeExtendedTimeDomain(period, some(3), some('Daily'), 'end');

		expect(unwrap(domain)).toEqual({ start: '2026-01-29', end: '2026-04-30' });
	});

	it('defaults to days when granularity is none', () => {
		const period = makePeriod();

		const domain = computeExtendedTimeDomain(period, some(10), none(), 'start');

		expect(unwrap(domain)).toEqual({ start: '2026-02-01', end: '2026-05-10' });
	});

	it('extends by months for monthly granularity', () => {
		const period = makePeriod({ start: '2026-02-15', end: '2026-03-10' });

		const domain = computeExtendedTimeDomain(period, some(1), some('Monthly'), 'start');

		expect(unwrap(domain)).toEqual({ start: '2026-02-15', end: '2026-04-10' });
	});

	it('keeps the period end as-is when aligning by end', () => {
		const period = makePeriod();

		const domain = computeExtendedTimeDomain(period, some(2), some('Weekly'), 'end');

		expect(unwrap(domain)).toEqual({ start: '2026-01-18', end: '2026-04-30' });
	});
});
