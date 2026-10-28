import type { HooperIndex } from '#lib/api/index.js';

export const SLEEP_WARNING_THRESHOLD = 5;
export const SLEEP_ALERT_THRESHOLD = 3;
export const FATIGUE_WARNING_THRESHOLD = 5;
export const FATIGUE_ALERT_THRESHOLD = 7;

export type FeedbackWarningSeverity = 'warning' | 'alert';

export interface FeedbackWarning {
	kind: 'sleep' | 'fatigue';
	severity: FeedbackWarningSeverity;
	label: string;
}

/**
 * Warning labels derived from the subjective feedback (Hooper index) recorded on
 * the activity's day. Missing measures never trigger a warning, and a measure
 * that crosses the alert threshold is reported once, as an alert.
 */
export const feedbackWarnings = (hooperIndex: HooperIndex): FeedbackWarning[] => {
	const warnings: FeedbackWarning[] = [];

	if (hooperIndex.sleep !== null && hooperIndex.sleep < SLEEP_WARNING_THRESHOLD) {
		warnings.push(
			hooperIndex.sleep < SLEEP_ALERT_THRESHOLD
				? { kind: 'sleep', severity: 'alert', label: `Very low sleep: ${hooperIndex.sleep}/10` }
				: { kind: 'sleep', severity: 'warning', label: `Low sleep: ${hooperIndex.sleep}/10` }
		);
	}

	if (hooperIndex.fatigue !== null && hooperIndex.fatigue > FATIGUE_WARNING_THRESHOLD) {
		warnings.push(
			hooperIndex.fatigue > FATIGUE_ALERT_THRESHOLD
				? {
						kind: 'fatigue',
						severity: 'alert',
						label: `Very high fatigue: ${hooperIndex.fatigue}/10`
					}
				: {
						kind: 'fatigue',
						severity: 'warning',
						label: `High fatigue: ${hooperIndex.fatigue}/10`
					}
		);
	}

	return warnings;
};
