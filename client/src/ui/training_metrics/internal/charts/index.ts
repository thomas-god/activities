import { scaleOrdinal, schemeCategory10 } from 'd3';
import type { TrainingMetric } from '$lib/api';
import { dayjs, formatDurationHoursMinutes, formatWeekInterval } from '$lib/duration';
import { isNone, isSome, some, type Option } from '$lib/Options';
import { paceInSecondToString } from '$lib/speed';
import type { TrainingMetricGranularity, TrainingMetricGroupByClause } from '$lib/trainingMetric';
import type { TimeDomain } from '$ui/training_metrics';

export const formatTooltipValue = (
	value: number,
	format: 'number' | 'duration' | 'pace',
	unit: string
): string => {
	if (format === 'duration') {
		return formatDurationHoursMinutes(value);
	}
	if (unit === 'activities') {
		return `${Math.round(value)} ${unit}`;
	}
	if (format === 'pace') {
		return `${paceInSecondToString(value)} /km`;
	}
	if (unit === 'kg') {
		return `${value.toFixed(2)} ${unit}`;
	}
	return `${value.toFixed(0)} ${unit}`;
};

/** Color of a metric value group, based on the group-by category. */
export const getGroupColor = (
	groupName: string,
	groupBy: Option<TrainingMetricGroupByClause>
): string | null => {
	if (isNone(groupBy)) return null;

	switch (groupBy.value) {
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

/** Stable colors for known sets of groups, so a missing group doesn't shift every color.

 * A chart whose groups all belong to a known set uses the set's fixed colors; any other
 * chart falls back to the default d3 ordinal scheme (colors assigned by sorted group order).
**/
const STABLE_GROUP_COLOR_SETS: Record<string, string>[] = [
	// Feedback (HooperIndexSource::All)
	{
		fatigue: 'var(--color-feedback-fatigue)',
		sleep: 'var(--color-feedback-sleep)',
		pain: 'var(--color-feedback-pain)',
		stress: 'var(--color-feedback-stress)',
		mood: 'var(--color-feedback-mood)'
	},
	// Weight and nutrition
	{
		'Total weight': 'var(--color-nutrition-total-weight)',
		Calories: 'var(--color-nutrition-calories)',
		Water: 'var(--color-nutrition-water)',
		Alcohol: 'var(--color-nutrition-alcohol)'
	}
];

export const getGroupColorScale = (groups: string[]): ((group: string) => string) => {
	for (const stableColors of STABLE_GROUP_COLOR_SETS) {
		if (groups.every((group) => group in stableColors)) {
			return (group: string) => stableColors[group];
		}
	}

	const fallback = scaleOrdinal(schemeCategory10).domain([...groups].sort());
	return (group: string) => fallback(group);
};

export type DisplayMode = 'relative' | 'absolute';
export type Point = { time: string; timestamp: number; group: string; value: number };

export const parseMetricIntoPoints = (
	metric: TrainingMetric['values'],
	timeDomain: TimeDomain,
	opts: {
		replaceNullValues: boolean;
	} = {
		replaceNullValues: true
	}
): { points: Point[]; times: string[] } => {
	const points: Point[] = [];
	const times: Set<string> = new Set();

	if (isSome(timeDomain)) {
		times.add(timeDomain.value.start);
	}

	for (const [group, granuleValues] of Object.entries(metric)) {
		for (const [time, value] of Object.entries(granuleValues)) {
			times.add(time);
			if (value === null) {
				if (opts.replaceNullValues) {
					points.push({ time, timestamp: dayjs(time).unix(), group, value: 0 });
				}
			} else {
				points.push({ time, timestamp: dayjs(time).unix(), group, value });
			}
		}
	}

	if (isSome(timeDomain) && timeDomain.value.end !== null) {
		times.add(timeDomain.value.end);
	}

	return { points, times: [...times.keys()].toSorted() };
};

/** Map a domain to match the border expected for a given granularity.

 * e.g. if granularity is Month make sure the resulting domain starts on the first day of a month,
 * same if granularity is weekly make sure the resulting domain starts on a Monday.
**/
export const mapDomainToGranularity = (
	domain: TimeDomain,
	granularity: TrainingMetricGranularity
): TimeDomain => {
	if (isNone(domain)) {
		return domain;
	}

	if (granularity === 'Daily') {
		return domain;
	} else if (granularity === 'Weekly') {
		return some({
			start: dayjs(domain.value.start).startOf('isoWeek').format('YYYY-MM-DD'),
			end:
				domain.value.end === null
					? null
					: dayjs(domain.value.end).startOf('isoWeek').format('YYYY-MM-DD')
		});
	} else if (granularity === 'Monthly') {
		return some({
			start: dayjs(domain.value.start).startOf('month').format('YYYY-MM-DD'),
			end:
				domain.value.end === null
					? null
					: dayjs(domain.value.end).startOf('month').format('YYYY-MM-DD')
		});
	}

	return domain;
};

export const buildAbsoluteTimeFormatter = (granularity: TrainingMetricGranularity) => {
	if (granularity === 'Monthly') {
		return (date: string, _idx: number) => {
			return dayjs(date).format('MMM YYYY');
		};
	}

	if (granularity === 'Weekly') {
		return (date: string) => {
			return formatWeekInterval(date);
		};
	}

	return (date: string, _idx: number) => {
		return dayjs(date).format('MMM D');
	};
};

export const buildRelativeTimeFormatter = (
	times: string[],
	granularity: TrainingMetricGranularity
) => {
	if (granularity === 'Monthly') {
		const mappedValues: Map<string, string> = new Map();
		for (const [idx, time] of times.entries()) {
			mappedValues.set(time, `Month ${idx + 1}`);
		}
		return (date: string, _idx: number) => {
			return mappedValues.get(date) ?? date;
		};
	}

	if (granularity === 'Weekly') {
		const mappedValues: Map<string, string> = new Map();
		for (const [idx, time] of times.entries()) {
			mappedValues.set(time, `Week ${idx + 1}`);
		}
		return (date: string, _idx: number) => {
			return mappedValues.get(date) ?? date;
		};
	}

	// Granularity === "daily"
	const mappedValues: Map<string, string> = new Map();
	for (const [idx, time] of times.entries()) {
		mappedValues.set(time, `Day ${idx + 1}`);
	}
	return (date: string, _idx: number) => {
		return mappedValues.get(date) ?? date;
	};
};

export const buildContinuousTimeRelativeFormatter = (domain: TimeDomain) => {
	if (isSome(domain)) {
		let start = dayjs(domain.value.start);
		const end = domain.value.end === null ? dayjs().startOf('day') : dayjs(domain.value.end);

		const days: Map<string, string> = new Map();
		let idx = 1;
		while (start <= end) {
			days.set(start.format('YYYY-MM-DD'), `Day ${idx}`);
			idx++;
			start = start.add(1, 'day');
		}

		return (date: d3.NumberValue, _idx: number) => {
			const day = dayjs.unix(date.valueOf());
			return `${days.get(day.format('YYYY-MM-DD')) ?? ''}`;
		};
	} else {
		return (date: d3.NumberValue, _idx: number) => dayjs.unix(date.valueOf()).format('YYYY-MM-DD');
	}
};
