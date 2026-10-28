import { describe, it, expect } from 'vitest';
import {
	FATIGUE_ALERT_THRESHOLD,
	FATIGUE_WARNING_THRESHOLD,
	SLEEP_ALERT_THRESHOLD,
	SLEEP_WARNING_THRESHOLD,
	feedbackWarnings
} from './feedbackWarnings';
import type { HooperIndex } from '#lib/api/index.js';

const hooperIndex = (values: Partial<HooperIndex> = {}): HooperIndex => ({
	fatigue: null,
	sleep: null,
	pain: null,
	stress: null,
	mood: null,
	...values
});

describe('feedbackWarnings', () => {
	it('returns no warning when measures are missing', () => {
		expect(feedbackWarnings(hooperIndex())).toEqual([]);
	});

	it('returns no warning when sleep and fatigue are within range', () => {
		expect(
			feedbackWarnings(
				hooperIndex({
					sleep: SLEEP_WARNING_THRESHOLD,
					fatigue: FATIGUE_WARNING_THRESHOLD
				})
			)
		).toEqual([]);
	});

	it('warns when sleep is below the warning threshold', () => {
		const sleep = SLEEP_WARNING_THRESHOLD - 1;

		expect(feedbackWarnings(hooperIndex({ sleep }))).toEqual([
			{ kind: 'sleep', severity: 'warning', label: `Low sleep: ${sleep}/10` }
		]);
	});

	it('alerts when sleep is below the alert threshold', () => {
		const sleep = SLEEP_ALERT_THRESHOLD - 1;

		expect(feedbackWarnings(hooperIndex({ sleep }))).toEqual([
			{ kind: 'sleep', severity: 'alert', label: `Very low sleep: ${sleep}/10` }
		]);
	});

	it('does not warn when sleep is at the alert threshold', () => {
		expect(feedbackWarnings(hooperIndex({ sleep: SLEEP_ALERT_THRESHOLD }))).toEqual([
			{
				kind: 'sleep',
				severity: 'warning',
				label: `Low sleep: ${SLEEP_ALERT_THRESHOLD}/10`
			}
		]);
	});

	it('warns when fatigue is above the warning threshold', () => {
		const fatigue = FATIGUE_WARNING_THRESHOLD + 1;

		expect(feedbackWarnings(hooperIndex({ fatigue }))).toEqual([
			{ kind: 'fatigue', severity: 'warning', label: `High fatigue: ${fatigue}/10` }
		]);
	});

	it('alerts when fatigue is above the alert threshold', () => {
		const fatigue = FATIGUE_ALERT_THRESHOLD + 1;

		expect(feedbackWarnings(hooperIndex({ fatigue }))).toEqual([
			{ kind: 'fatigue', severity: 'alert', label: `Very high fatigue: ${fatigue}/10` }
		]);
	});

	it('does not alert when fatigue is at the alert threshold', () => {
		expect(feedbackWarnings(hooperIndex({ fatigue: FATIGUE_ALERT_THRESHOLD }))).toEqual([
			{
				kind: 'fatigue',
				severity: 'warning',
				label: `High fatigue: ${FATIGUE_ALERT_THRESHOLD}/10`
			}
		]);
	});

	it('returns the sleep warning before the fatigue warning', () => {
		const warnings = feedbackWarnings(hooperIndex({ sleep: 2, fatigue: 9 }));

		expect(warnings.map((warning) => warning.kind)).toEqual(['sleep', 'fatigue']);
		expect(warnings.map((warning) => warning.severity)).toEqual(['alert', 'alert']);
	});
});
