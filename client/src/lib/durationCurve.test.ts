import { describe, expect, it } from 'vitest';
import {
	computeDurationCurveRecords,
	formatCurveDuration,
	formatCurveValue,
	type DurationCurveReference
} from './durationCurve';

describe('computeDurationCurveRecords', () => {
	it('reports a record for each duration beating the reference', () => {
		const activity = [260, 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150];
		const references: DurationCurveReference[] = [
			{
				label: '12-week',
				values: [250, 250, 240, 220, 220, 210, 200, 180, 180, 170, 160, 150]
			}
		];

		const groups = computeDurationCurveRecords(activity, references, 'W');

		expect(groups).toHaveLength(1);
		expect(groups[0].label).toBe('12-week');
		// Durations with an equal or lower value are not reported.
		expect(groups[0].records).toEqual([
			{ duration: 5, previousValue: 250, newValue: 260 },
			{ duration: 60, previousValue: 220, newValue: 230 },
			{ duration: 1200, previousValue: 180, newValue: 190 }
		]);
	});

	it('does not report a record when values are equal', () => {
		const activity = [250];
		const references: DurationCurveReference[] = [{ label: '12-week', values: [250] }];

		expect(computeDurationCurveRecords(activity, references, 'W')).toEqual([]);
	});

	it('does not report a record when values round to the same displayed value', () => {
		const activity = [161.2];
		const references: DurationCurveReference[] = [{ label: '12-week', values: [160.6] }];

		expect(computeDurationCurveRecords(activity, references, 'W')).toEqual([]);
	});

	it('reports a record when rounded values differ', () => {
		const activity = [160.6];
		const references: DurationCurveReference[] = [{ label: '12-week', values: [159.6] }];

		const groups = computeDurationCurveRecords(activity, references, 'W');

		expect(groups[0].records).toEqual([{ duration: 5, previousValue: 159.6, newValue: 160.6 }]);
	});

	it('compares other units at a single decimal', () => {
		const activity = [15.34];
		const references: DurationCurveReference[] = [{ label: '12-week', values: [15.31] }];

		expect(computeDurationCurveRecords(activity, references, 'km/h')).toEqual([]);
	});

	it('reports a record when the reference has no value for a duration', () => {
		const activity = [null, null, null, null, null, 210];
		const references: DurationCurveReference[] = [
			{ label: 'all-time', values: [200, null, null, null, null, null] }
		];

		const groups = computeDurationCurveRecords(activity, references, 'W');

		expect(groups[0].records).toEqual([{ duration: 300, previousValue: null, newValue: 210 }]);
	});

	it('ignores durations missing from the activity curve', () => {
		const activity = [null, null, null, null, null, null, null, null, null, null, null, null];
		const references: DurationCurveReference[] = [
			{ label: '12-week', values: new Array(12).fill(100) }
		];

		expect(computeDurationCurveRecords(activity, references, 'W')).toEqual([]);
	});

	it('drops reference groups without any record', () => {
		const activity = [100, 100];
		const references: DurationCurveReference[] = [
			{ label: '12-week', values: [200, 200] },
			{ label: 'all-time', values: [50, 50] }
		];

		const groups = computeDurationCurveRecords(activity, references, 'W');

		expect(groups).toHaveLength(1);
		expect(groups[0].label).toBe('all-time');
	});

	it('returns one group per reference, in order', () => {
		const activity = [300, 300];
		const references: DurationCurveReference[] = [
			{ label: '12-week', values: [250, 250] },
			{ label: 'all-time', values: [290, 290] }
		];

		const groups = computeDurationCurveRecords(activity, references, 'W');

		expect(groups.map((group) => group.label)).toEqual(['12-week', 'all-time']);
		expect(groups[0].records).toHaveLength(2);
		expect(groups[1].records).toHaveLength(2);
	});
});

describe('formatCurveDuration', () => {
	it('formats seconds, minutes and hours', () => {
		expect(formatCurveDuration(5)).toBe('5 s');
		expect(formatCurveDuration(30)).toBe('30 s');
		expect(formatCurveDuration(60)).toBe('1 min');
		expect(formatCurveDuration(300)).toBe('5 min');
		expect(formatCurveDuration(3600)).toBe('1 h');
		expect(formatCurveDuration(18000)).toBe('5 h');
	});
});

describe('formatCurveValue', () => {
	it('rounds watts to whole numbers', () => {
		expect(formatCurveValue(129.6, 'W')).toBe('130 W');
	});

	it('keeps one decimal for other units', () => {
		expect(formatCurveValue(15.34, 'km/h')).toBe('15.3 km/h');
	});
});
