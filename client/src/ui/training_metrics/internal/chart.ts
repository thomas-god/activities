import { formatDurationCompactWithUnits } from '$lib/duration';
import { paceInSecondToString } from '$lib/speed';

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
