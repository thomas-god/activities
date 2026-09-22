import { describe, expect, it } from 'vitest';

import { dayjs } from '$lib/duration';
import { isNone, none, some } from '$lib/Options';

import {
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

		expect(mapDomainToGranularity(domain, some('Weekly'))).toBe(domain);
	});

	it('returns the domain unchanged when the granularity is none', () => {
		const domain = some({ start: '2026-01-01', end: null });

		expect(isNone(mapDomainToGranularity(domain, none()))).toBe(false);
		expect(mapDomainToGranularity(domain, none())).toEqual(domain);
	});

	it('returns the domain unchanged for Daily granularity', () => {
		const domain = some({ start: '2026-01-01', end: '2026-01-31' });

		expect(mapDomainToGranularity(domain, some('Daily'))).toEqual(
			some({ start: '2026-01-01', end: '2026-01-31' })
		);
	});

	it('snaps the domain to iso week starts for Weekly granularity', () => {
		// 2026-01-01 is a Thursday, its iso week starts on Monday 2025-12-29.
		const domain = some({ start: '2026-01-01', end: '2026-01-10' });

		expect(mapDomainToGranularity(domain, some('Weekly'))).toEqual(
			some({ start: '2025-12-29', end: '2026-01-05' })
		);
	});

	it('keeps a null end when snapping to weeks', () => {
		const domain = some({ start: '2026-01-01', end: null });

		expect(mapDomainToGranularity(domain, some('Weekly'))).toEqual(
			some({ start: '2025-12-29', end: null })
		);
	});

	it('snaps the domain to month starts for Monthly granularity', () => {
		const domain = some({ start: '2026-01-15', end: '2026-03-10' });

		expect(mapDomainToGranularity(domain, some('Monthly'))).toEqual(
			some({ start: '2026-01-01', end: '2026-03-01' })
		);
	});

	it('keeps a null end when snapping to months', () => {
		const domain = some({ start: '2026-01-15', end: null });

		expect(mapDomainToGranularity(domain, some('Monthly'))).toEqual(
			some({ start: '2026-01-01', end: null })
		);
	});
});
