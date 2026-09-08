import { formatDurationCompactWithUnits } from '$lib/duration';
import { paceInSecondToString } from '$lib/speed';
import type { TrainingMetricGroupByClause } from '$lib/trainingMetric';

export const formatTooltipValue = (
	value: number,
	format: 'number' | 'duration' | 'pace',
	unit: string
): string => {
	if (format === 'duration') {
		return formatDurationCompactWithUnits(value);
	}
	if (unit === 'activities') {
		return `${Math.round(value)} ${unit}`;
	}
	if (format === 'pace') {
		return `${paceInSecondToString(value)} /km`;
	}
	return `${value.toFixed(0)} ${unit}`;
};

/** Color of a metric value group, based on the group-by category. */
export const getGroupColor = (
	groupName: string,
	groupBy: TrainingMetricGroupByClause | null
): string | null => {
	if (!groupBy) return null;

	switch (groupBy) {
		case 'RpeRange':
			if (groupName === 'Easy') return 'var(--color-rpe-easy)';
			if (groupName === 'Moderate') return 'var(--color-rpe-moderate)';
			if (groupName === 'Hard') return 'var(--color-rpe-hard)';
			if (groupName === 'Very Hard') return 'var(--color-rpe-very-hard)';
			if (groupName === 'Maximum Effort') return 'var(--color-rpe-max)';
			if (groupName === 'Other') return 'var(--color-rpe-other)';
			return null;

		case 'WorkoutType':
			if (groupName === 'Easy') return 'var(--color-workout-easy)';
			if (groupName === 'Tempo') return 'var(--color-workout-tempo)';
			if (groupName === 'Intervals') return 'var(--color-workout-intervals)';
			if (groupName === 'Long Run') return 'var(--color-workout-long-run)';
			if (groupName === 'Race') return 'var(--color-workout-race)';
			if (groupName === 'Cross Training') return 'var(--color-workout-cross-training)';
			if (groupName === 'Other') return 'var(--color-workout-other)';
			return null;

		default:
			return null;
	}
};
