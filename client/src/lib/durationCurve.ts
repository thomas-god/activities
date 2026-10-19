/**
 * Fixed set of durations (in seconds) at which duration curves are sampled.
 * Must stay in sync with the server's `DURATION_CURVE_DURATIONS_SECOND`.
 */
export const DURATION_CURVE_DURATIONS = [
	5, 10, 30, 60, 120, 300, 600, 1200, 1800, 3600, 7200, 18000
];

/**
 * A reference curve an activity's own duration curve can be compared against,
 * e.g. the best curve over the 12 weeks preceding the activity. Values are
 * indexed like `DURATION_CURVE_DURATIONS`; `null` means the reference does not
 * cover that duration.
 */
export interface DurationCurveReference {
	/** Short data label of the reference span, e.g. `12-week`. */
	label: string;
	values: (number | null)[];
}

/** A single duration for which the activity outperformed a reference curve. */
export interface DurationCurveRecord {
	duration: number;
	/** Best value of the reference curve, `null` when it had none. */
	previousValue: number | null;
	newValue: number;
}

export interface DurationCurveRecordGroup {
	label: string;
	records: DurationCurveRecord[];
}

/**
 * Computes the new records set by an activity, one entry per duration where the
 * activity's curve beats a reference curve.
 *
 * Duration curves use a "higher is better" convention on the client: power is
 * expressed in watts and pace in km/h, so an improvement is a strictly greater
 * value. Durations missing from the activity (or for which the activity does
 * not beat the reference) are ignored. A reference value of `null` is treated
 * as an unset benchmark, so the activity value always yields a record.
 *
 * Values are compared at the precision they are displayed with (see
 * `roundCurveValue`), so a barely higher raw value that rounds to the same
 * number as the reference does not produce a spurious record.
 */
export function computeDurationCurveRecords(
	activityValues: (number | null)[],
	references: DurationCurveReference[],
	unit: string
): DurationCurveRecordGroup[] {
	const groups: DurationCurveRecordGroup[] = [];

	for (const reference of references) {
		const records: DurationCurveRecord[] = [];

		for (let i = 0; i < DURATION_CURVE_DURATIONS.length; i++) {
			const newValue = activityValues[i];
			if (newValue === null || newValue === undefined) {
				continue;
			}

			const previousValue = reference.values[i] ?? null;
			if (
				previousValue !== null &&
				roundCurveValue(newValue, unit) <= roundCurveValue(previousValue, unit)
			) {
				continue;
			}

			records.push({
				duration: DURATION_CURVE_DURATIONS[i],
				previousValue,
				newValue
			});
		}

		if (records.length > 0) {
			groups.push({ label: reference.label, records });
		}
	}

	return groups;
}

/** Formats a duration in seconds as a short human-readable label, e.g. `5 min`. */
export function formatCurveDuration(duration: number): string {
	if (duration < 60) {
		return `${duration} s`;
	}
	if (duration < 3600) {
		return `${duration / 60} min`;
	}
	return `${duration / 3600} h`;
}

/**
 * Rounds a duration curve value to the precision it is displayed with. Watt
 * values are whole numbers, other units (e.g. km/h) keep a single decimal.
 */
export function roundCurveValue(value: number, unit: string): number {
	return unit === 'W' ? Math.round(value) : Math.round(value * 10) / 10;
}

/**
 * Formats a duration curve value with its unit, rounded as done by
 * `roundCurveValue`.
 */
export function formatCurveValue(value: number, unit: string): string {
	return `${roundCurveValue(value, unit)} ${unit}`;
}
