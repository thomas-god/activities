import { describe, expect, it } from 'vitest';

import type { Activity } from '$lib/api/activities';

import {
	formatDistance,
	formatElevation,
	formatPeriodDuration,
	periodActivitiesSummary
} from './trainingPeriod';

// fr-fr number grouping uses narrow no-break spaces
const NBSP = '\u202f';

const makeActivity = (metrics: Record<string, { value: number; unit: string }>): Activity => ({
	id: 'activity-1',
	name: null,
	sport: 'Running',
	sport_category: 'Running',
	start_time: '2026-01-05T10:00:00+01:00',
	rpe: null,
	workout_type: null,
	feedback: null,
	nutrition: null,
	metrics
});

describe('periodActivitiesSummary', () => {
	it('sums activity metrics and counts activities', () => {
		const activities = [
			makeActivity({
				ActiveDuration: { value: 3600, unit: 's' },
				Distance: { value: 10000, unit: 'm' },
				Elevation: { value: 250, unit: 'm' }
			}),
			makeActivity({
				ActiveDuration: { value: 1800, unit: 's' },
				Distance: { value: 5200.5, unit: 'm' },
				Elevation: { value: 120.4, unit: 'm' }
			})
		];

		expect(periodActivitiesSummary(activities)).toEqual({
			count: 2,
			duration: 5400,
			distance: 15200.5,
			elevation: 370.4
		});
	});

	it('treats missing metrics as zero', () => {
		const activities = [
			makeActivity({ ActiveDuration: { value: 600, unit: 's' } }),
			makeActivity({})
		];

		expect(periodActivitiesSummary(activities)).toEqual({
			count: 2,
			duration: 600,
			distance: 0,
			elevation: 0
		});
	});

	it('returns zeroed summary for an empty period', () => {
		expect(periodActivitiesSummary([])).toEqual({
			count: 0,
			duration: 0,
			distance: 0,
			elevation: 0
		});
	});
});

describe('formatPeriodDuration', () => {
	it('formats a single day', () => {
		expect(formatPeriodDuration('2026-01-05', '2026-01-05')).toBe('1 day');
	});

	it('formats less than a week in days (end date inclusive)', () => {
		expect(formatPeriodDuration('2026-01-05', '2026-01-06')).toBe('2 days');
		expect(formatPeriodDuration('2026-01-05', '2026-01-10')).toBe('6 days');
	});

	it('formats whole weeks', () => {
		expect(formatPeriodDuration('2026-01-05', '2026-01-11')).toBe('1 week');
		expect(formatPeriodDuration('2026-01-05', '2026-01-18')).toBe('2 weeks');
	});

	it('formats weeks and remaining days', () => {
		expect(formatPeriodDuration('2026-01-05', '2026-01-26')).toBe('3 weeks 1 day');
	});
});

describe('formatDistance', () => {
	it('formats zero without decimals', () => {
		expect(formatDistance(0)).toBe('0 km');
	});

	it('formats meters as rounded kilometers', () => {
		expect(formatDistance(10000)).toBe('10 km');
		expect(formatDistance(15200.5)).toBe('15 km');
	});

	it('groups thousands', () => {
		expect(formatDistance(1842000)).toBe(`1${NBSP}842 km`);
	});
});

describe('formatElevation', () => {
	it('formats zero without decimals', () => {
		expect(formatElevation(0)).toBe('0 m');
	});

	it('formats rounded meters', () => {
		expect(formatElevation(1520.4)).toBe(`1${NBSP}520 m`);
	});
});
