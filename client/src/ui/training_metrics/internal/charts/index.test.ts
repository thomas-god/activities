import { describe, expect, it } from 'vitest';

import { dayjs } from '$lib/duration';
import { none, some } from '$lib/Options';

import {
	buildAbsoluteTimeFormatter,
	buildContinuousTimeRelativeFormatter,
	buildRelativeTimeFormatter,
	formatTooltipValue,
	mapDomainToGranularity,
	parseMetricIntoPoints,
	type Point,
	type TimeDomain
} from './index';

describe('formatTooltipValue', () => {
	it('formats duration values with compact units', () => {
		expect(formatTooltipValue(90, 'duration', 's')).toBe('1m');
		expect(formatTooltipValue(0, 'duration', 's')).toBe('0m');
		expect(formatTooltipValue(90000, 'duration', 's')).toBe('1d01h');
	});

	it('rounds number values for activities', () => {
		expect(formatTooltipValue(3.2, 'number', 'activities')).toBe('3 activities');
		expect(formatTooltipValue(10.7, 'number', 'activities')).toBe('11 activities');
	});

	it('formats pace values as min/km', () => {
		expect(formatTooltipValue(300, 'pace', 's')).toBe('5:00 /km');
	});

	it('formats generic number values with the unit', () => {
		expect(formatTooltipValue(12.4, 'number', 'km')).toBe('12 km');
		expect(formatTooltipValue(0, 'number', 'km')).toBe('0 km');
	});

	it('formats unknown format values as numbers with the unit', () => {
		expect(formatTooltipValue(42.6, 'pace', 'activities')).toBe('43 activities');
	});
});

describe('parseMetricIntoPoints', () => {
	it('parses metric values into points and collects all times', () => {
		const metric = {
			Running: { '2026-01-01': 10, '2026-01-08': 20 },
			Cycling: { '2026-01-01': 5 }
		};

		const { points, times } = parseMetricIntoPoints(metric, none());

		expect(times).toEqual(['2026-01-01', '2026-01-08']);
		expect(points).toEqual(
			expect.arrayContaining<Point[]>([
				expect.objectContaining({ time: '2026-01-01', group: 'Running', value: 10 }),
				expect.objectContaining({ time: '2026-01-08', group: 'Running', value: 20 }),
				expect.objectContaining({ time: '2026-01-01', group: 'Cycling', value: 5 })
			])
		);
		expect(points).toHaveLength(3);
	});

	it('computes timestamps from the time', () => {
		const metric = { Running: { '2026-01-01': 10 } };
		const { points } = parseMetricIntoPoints(metric, none());

		expect(points).toHaveLength(1);
		expect(points[0].timestamp).toBe(dayjs('2026-01-01').unix());
	});

	it('includes time domain start and end in the times', () => {
		const metric = { Running: { '2026-01-08': 10 } };
		const timeDomain = some({ start: '2026-01-01', end: '2026-01-15' });

		const { points, times } = parseMetricIntoPoints(metric, timeDomain);

		expect(times).toEqual(['2026-01-01', '2026-01-08', '2026-01-15']);
		expect(points).toHaveLength(1);
	});

	it('includes only the start when the time domain end is null', () => {
		const metric = { Running: { '2026-01-08': 10 } };
		const timeDomain = some({ start: '2026-01-01', end: null });

		const { times } = parseMetricIntoPoints(metric, timeDomain);

		expect(times).toEqual(['2026-01-01', '2026-01-08']);
	});

	it('converts null values to 0 by default', () => {
		const metric = { Running: { '2026-01-01': 10, '2026-01-08': null } };

		const { points } = parseMetricIntoPoints(metric, none());

		expect(points).toHaveLength(2);
		expect(points.map((p) => p.value)).toEqual([10, 0]);
	});

	it('drops null values when requested', () => {
		const metric = { Running: { '2026-01-01': 10, '2026-01-08': null } };

		const { points, times } = parseMetricIntoPoints(metric, none(), { replaceNullValues: false });

		expect(points).toEqual([
			{ time: '2026-01-01', timestamp: points[0].timestamp, group: 'Running', value: 10 }
		]);
		// Null times are still collected even when their points are dropped.
		expect(times).toEqual(['2026-01-01', '2026-01-08']);
	});
});

describe('mapDomainToGranularity', () => {
	it('returns the domain unchanged when the domain is none', () => {
		const domain: TimeDomain = none();

		expect(mapDomainToGranularity(domain, 'Weekly')).toBe(domain);
	});

	it('returns the domain unchanged for Daily granularity', () => {
		const domain = some({ start: '2026-01-01', end: '2026-01-31' });

		expect(mapDomainToGranularity(domain, 'Daily')).toEqual(
			some({ start: '2026-01-01', end: '2026-01-31' })
		);
	});

	it('snaps the domain to iso week starts for Weekly granularity', () => {
		// 2026-01-01 is a Thursday, its iso week starts on Monday 2025-12-29.
		const domain = some({ start: '2026-01-01', end: '2026-01-10' });

		expect(mapDomainToGranularity(domain, 'Weekly')).toEqual(
			some({ start: '2025-12-29', end: '2026-01-05' })
		);
	});

	it('keeps a null end when snapping to weeks', () => {
		const domain = some({ start: '2026-01-01', end: null });

		expect(mapDomainToGranularity(domain, 'Weekly')).toEqual(
			some({ start: '2025-12-29', end: null })
		);
	});

	it('snaps the domain to month starts for Monthly granularity', () => {
		const domain = some({ start: '2026-01-15', end: '2026-03-10' });

		expect(mapDomainToGranularity(domain, 'Monthly')).toEqual(
			some({ start: '2026-01-01', end: '2026-03-01' })
		);
	});

	it('keeps a null end when snapping to months', () => {
		const domain = some({ start: '2026-01-15', end: null });

		expect(mapDomainToGranularity(domain, 'Monthly')).toEqual(
			some({ start: '2026-01-01', end: null })
		);
	});
});

describe('buildAbsoluteTimeFormatter', () => {
	it('formats monthly tick dates as MMM YYYY', () => {
		const formatter = buildAbsoluteTimeFormatter('Monthly');

		expect(formatter('2026-01-15', 0)).toBe('Jan 2026');
		expect(formatter('2026-03-01', 1)).toBe('Mar 2026');
	});

	it('formats weekly tick dates as week intervals', () => {
		const formatter = buildAbsoluteTimeFormatter('Weekly');

		// 2026-01-05 is a Monday, so its interval spans Jan 5 to Jan 11.
		expect(formatter('2026-01-05', 0)).toBe('Jan 5-11');

		// 2026-01-26 is a Monday whose iso week ends in the next month.
		expect(formatter('2026-01-26', 1)).toBe('Jan 26-Feb 1');
	});

	it('formats daily tick dates as MMM D', () => {
		const formatter = buildAbsoluteTimeFormatter('Daily');

		expect(formatter('2026-01-05', 0)).toBe('Jan 5');
		expect(formatter('2026-12-31', 1)).toBe('Dec 31');
	});
});

describe('buildRelativeTimeFormatter', () => {
	it('maps monthly times to 1-based month labels', () => {
		const formatter = buildRelativeTimeFormatter(['2026-01-01', '2026-02-01'], 'Monthly');

		expect(formatter('2026-01-01', 0)).toBe('Month 1');
		expect(formatter('2026-02-01', 1)).toBe('Month 2');
	});

	it('maps weekly times to 1-based week labels', () => {
		const formatter = buildRelativeTimeFormatter(['2026-01-05', '2026-01-12'], 'Weekly');

		expect(formatter('2026-01-05', 0)).toBe('Week 1');
		expect(formatter('2026-01-12', 1)).toBe('Week 2');
	});

	it('maps daily times to 1-based day labels', () => {
		const formatter = buildRelativeTimeFormatter(['2026-01-01', '2026-01-02'], 'Daily');

		expect(formatter('2026-01-01', 0)).toBe('Day 1');
		expect(formatter('2026-01-02', 1)).toBe('Day 2');
	});

	it('falls back to the raw date for unknown times', () => {
		const formatter = buildRelativeTimeFormatter(['2026-01-01'], 'Daily');

		expect(formatter('2026-06-15', 1)).toBe('2026-06-15');
	});
});

describe('buildContinuousTimeRelativeFormatter', () => {
	it('labels each day of the domain with a 1-based day number', () => {
		const formatter = buildContinuousTimeRelativeFormatter(
			some({ start: '2026-01-01', end: '2026-01-03' })
		);

		expect(formatter(dayjs('2026-01-01').unix(), 0)).toBe('Day 1');
		expect(formatter(dayjs('2026-01-02').unix(), 1)).toBe('Day 2');
		expect(formatter(dayjs('2026-01-03').unix(), 2)).toBe('Day 3');
	});

	it('uses today as the end when the domain end is null', () => {
		const today = dayjs().startOf('day');
		const formatter = buildContinuousTimeRelativeFormatter(some({ start: '2026-01-01', end: null }));

		// The label is relative to the start, not to today.
		expect(formatter(dayjs('2026-01-01').unix(), 0)).toBe('Day 1');
		// Days beyond today are not labelled.
		expect(formatter(today.add(1, 'day').unix(), 999)).toBe('');
	});

	it('returns an empty label for dates outside the domain', () => {
		const formatter = buildContinuousTimeRelativeFormatter(
			some({ start: '2026-01-01', end: '2026-01-03' })
		);

		expect(formatter(dayjs('2025-12-31').unix(), 0)).toBe('');
		expect(formatter(dayjs('2026-01-04').unix(), 3)).toBe('');
	});

	it('formats dates as YYYY-MM-DD when the domain is none', () => {
		const formatter = buildContinuousTimeRelativeFormatter(none());

		expect(formatter(dayjs('2026-01-02').unix(), 0)).toBe('2026-01-02');
	});
});
