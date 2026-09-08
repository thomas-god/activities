import { getBonkStatusLabel, bonkStatusToAPI, type BonkStatus } from './nutrition';
import { sportCategoryDisplay, sportDisplay, type Sport, type SportCategory } from './sport';
import { workoutTypeDisplay, workoutTypeToAPI, type WorkoutType } from './workout-type';
import type { RPEValue } from './rpe';
import { dayjs } from './duration';
import type {
	TrainingMetric,
	TrainingMetricBasePayload,
	PreviewTrainingMetricPayload,
	TrainingPeriodDetails
} from '$lib/api';

export const trainingMetricGranularities = ['Daily', 'Weekly', 'Monthly'] as const;
export type TrainingMetricGranularity = (typeof trainingMetricGranularities)[number];

export const trainingMetricTemplateCategories = [
	'Duration',
	'Calories',
	'Elevation',
	'Distance',
	'Speed',
	'Power',
	'HeartRate',
	'Cadence',
	'Altitude',
	'Pace',
	'Other'
] as const;
export type TrainingMetricTemplateCategory = (typeof trainingMetricTemplateCategories)[number];

export const trainingMetricAggregateFunctions = [
	'Min',
	'Max',
	'Average',
	'Sum',
	'NumberOfActivities'
] as const;
export type TrainingMetricAggregateFunction = (typeof trainingMetricAggregateFunctions)[number];

export const trainingMetricGroupByClauses = [
	'Sport',
	'SportCategory',
	'WorkoutType',
	'RpeRange',
	'Bonked'
] as const;
export type TrainingMetricGroupByClause = (typeof trainingMetricGroupByClauses)[number];

export const aggregateFunctionDisplay: Record<TrainingMetricAggregateFunction, string> = {
	Average: 'average',
	Max: 'maximum',
	Min: 'minimum',
	Sum: 'total',
	NumberOfActivities: 'number of activities'
};

export const groupByClauseDisplay = (clause: TrainingMetricGroupByClause): string => {
	switch (clause) {
		case 'Sport':
			return 'sport';
		case 'SportCategory':
			return 'sport category';
		case 'WorkoutType':
			return 'workout type';
		case 'RpeRange':
			return 'RPE range';
		case 'Bonked':
			return 'bonked status';
	}
};

export const rpeRanges = ['easy', 'moderate', 'hard', 'very_hard', 'maximum'] as const;
export type RpeRange = (typeof rpeRanges)[number];

export const rpeRangeDisplay = (range: RpeRange): string => {
	switch (range) {
		case 'easy':
			return 'Easy';
		case 'moderate':
			return 'Moderate';
		case 'hard':
			return 'Hard';
		case 'very_hard':
			return 'Very Hard';
		case 'maximum':
			return 'Maximum Effort';
	}
};

export const displayGroupName = (
	group: string,
	groupBy: TrainingMetricGroupByClause | null
): string => {
	if (group === 'Other') {
		return 'Other';
	}

	if (groupBy === null) {
		return group;
	}

	switch (groupBy) {
		case 'Sport':
			return sportDisplay(group as Sport);
		case 'SportCategory':
			return sportCategoryDisplay(group as SportCategory);
		case 'RpeRange':
			return rpeRangeDisplay(group as RpeRange);
		case 'WorkoutType':
			return workoutTypeDisplay(group as WorkoutType);
		case 'Bonked':
			return getBonkStatusLabel(group as BonkStatus);
	}
};

export const metricValuesDisplayFormat = (metric: {
	aggregate: string | null;
	unit: string;
}): 'number' | 'duration' | 'pace' => {
	if (metric.aggregate === 'NumberOfActivities') return 'number';
	if (metric.unit === 's') return 'duration';
	if (metric.unit === 's/km') return 'pace';
	return 'number';
};

// =============================================================================
// Comparing training periods' metrics
// =============================================================================

/** How the compared periods' time axes are aligned on the comparison charts. */
export type CompareAlignment = 'start' | 'end';

/** Where a compared metric definition comes from. */
export type CompareMetricSource = 'default' | 'first' | 'second' | 'both';

/**
 * One entry of the metric comparison: a single metric definition which is
 * computed over each compared period's own date range. `label` is null when
 * the definition has no name and must be composed for display.
 */
export type CompareMetricDefinition = {
	key: string;
	label: string | null;
	source: CompareMetricSource;
	base: TrainingMetricBasePayload;
};

/**
 * Identity of a metric definition, used to deduplicate metrics that exist in
 * both periods. Deliberately excludes the display name and target.
 */
export const metricDefinitionKey = (metric: TrainingMetric): string =>
	JSON.stringify({
		metric: metric.metric,
		granularity: metric.granularity,
		aggregate: metric.aggregate,
		group_by: metric.group_by,
		sports: metric.sports,
		workout_types: metric.workout_types,
		rpes: metric.rpes,
		bonked: metric.bonked
	});

const periodSportFilters = (
	period: Pick<TrainingPeriodDetails, 'sports'>
): ({ Sport: Sport } | { SportCategory: SportCategory })[] | undefined => {
	if (period.sports.categories.length === 0 && period.sports.sports.length === 0) {
		return undefined;
	}

	return [
		...period.sports.categories.map((category) => ({ SportCategory: category })),
		...period.sports.sports.map((sport) => ({ Sport: sport }))
	];
};

/**
 * Builds the comparable definition of a saved metric. Filters keep only the
 * explicitly set parts: missing sports mean "the period's own sports" (like
 * the server-side computation of period metrics) and are filled per compared
 * period by `compareMetricPreviewPayload`.
 */
export const extractBaseDefinitionFromMetric = (
	metric: TrainingMetric
): TrainingMetricBasePayload => {
	const payload: TrainingMetricBasePayload = { metric: metric.metric };

	if (metric.granularity !== null && metric.aggregate !== null) {
		payload.window = {
			granularity: metric.granularity,
			aggregate: metric.aggregate
		};
		if (metric.group_by !== null) {
			payload.window.group_by = metric.group_by;
		}
	}

	if (
		metric.sports !== null ||
		metric.workout_types !== null ||
		metric.rpes !== null ||
		metric.bonked !== null
	) {
		// Filter values use their API representation (like `fieldsAsPayload` sends
		// them): workout types and bonk status go through the *ToAPI converters.
		// The `TrainingMetricBasePayload` filter types don't reflect that wire
		// format, hence the local type and final cast.
		const filters: {
			sports?: ({ Sport: Sport } | { SportCategory: SportCategory })[];
			workout_types?: string[];
			rpes?: number[];
			bonked?: string;
		} = {};

		if (metric.sports !== null) {
			if (metric.sports.categories.length > 0 || metric.sports.sports.length > 0) {
				filters.sports = [
					...metric.sports.categories.map((category) => ({ SportCategory: category })),
					...metric.sports.sports.map((sport) => ({ Sport: sport }))
				];
			}
		}
		if (metric.workout_types !== null && metric.workout_types.length > 0) {
			filters.workout_types = metric.workout_types.map(workoutTypeToAPI);
		}
		if (metric.rpes !== null && metric.rpes.length > 0) {
			filters.rpes = metric.rpes.map((rpe) => rpe as RPEValue);
		}
		if (metric.bonked !== null) {
			filters.bonked = bonkStatusToAPI(metric.bonked);
		}

		payload.filters = filters as TrainingMetricBasePayload['filters'];
	}

	if (metric.show_average !== null) {
		payload.summary = { average: { include_zeros: metric.show_average.include_zeros } };
	}

	if (metric.target !== null) {
		payload.target = metric.target;
	}

	return payload;
};

/**
 * Final preview payload for one side of a comparison: the definition computed
 * over the given period's own date range. Sports filters default to the
 * period's sports so each side stays faithful to what the period page shows.
 */
export const metricPreviewPayload = (
	definition: CompareMetricDefinition,
	period: Pick<TrainingPeriodDetails, 'start' | 'end' | 'sports'>
): PreviewTrainingMetricPayload => {
	const filters =
		definition.base.filters !== undefined && definition.base.filters.sports !== undefined
			? definition.base.filters
			: { ...definition.base.filters, sports: periodSportFilters(period) };

	return {
		...definition.base,
		filters,
		...periodMetricRange(period)
	};
};

/**
 * The date range over which a metric is computed for a period. The range end
 * is extended by one day so the period's last day is included (now when the
 * period is ongoing).
 */
export const periodMetricRange = (
	period: Pick<TrainingPeriodDetails, 'start' | 'end'>
): { start: string; end: string } => ({
	start: dayjs(period.start).format('YYYY-MM-DDTHH:mm:ssZ'),
	end: (period.end === null ? dayjs() : dayjs(period.end))
		.add(1, 'day')
		.format('YYYY-MM-DDTHH:mm:ssZ')
});

/**
 * The date both compared charts align their buckets to for a period.
 * Aligning by end requires a known end date; ongoing periods fall back to start.
 */
export const compareAnchor = (
	period: Pick<TrainingPeriodDetails, 'start' | 'end'>,
	alignBy: CompareAlignment
): string => (alignBy === 'end' && period.end !== null ? period.end : period.start);

/**
 * Position of a metric value's bucket on the aligned axis: the number of
 * granules between the bucket and the anchor date (negative when the bucket
 * precedes the anchor).
 */
export const bucketOffset = (
	bucketDate: string,
	anchor: string,
	granularity: TrainingMetricGranularity
): number => {
	const date = dayjs(bucketDate);
	const anchorDate = dayjs(anchor);
	switch (granularity) {
		case 'Weekly':
			return date.startOf('isoWeek').diff(anchorDate.startOf('isoWeek'), 'week');
		case 'Monthly':
			return date.startOf('month').diff(anchorDate.startOf('month'), 'month');
		default:
			return date.startOf('day').diff(anchorDate.startOf('day'), 'day');
	}
};

/**
 * Relative label of a bucket on the aligned axis: the bucket's ordinal
 * position on the compared charts' shared domain, so both charts use the same
 * labels ("Week 3" is the third timeslot of the comparison, whatever the
 * alignment mode).
 */
export const relativeBucketLabel = (
	offset: number,
	firstOffset: number,
	granularity: TrainingMetricGranularity
): string => {
	const unit = granularity === 'Daily' ? 'Day' : granularity === 'Weekly' ? 'Week' : 'Month';
	return `${unit} ${offset - firstOffset + 1}`;
};

/**
 * The shared x-axis domain of a metric comparison: the sorted union of the
 * compared periods' bucket offsets. Sharing the domain aligns bucket
 * positions across the two charts (ticks, labels and synced crosshair), each
 * chart only drawing bars where its period has data.
 */
export const compareBucketDomain = (
	metrics: (TrainingMetric | undefined)[],
	anchors: (string | undefined)[]
): number[] => {
	const offsets = new Set<number>();
	metrics.forEach((metric, idx) => {
		const anchor = anchors[idx];
		if (metric === undefined || metric.granularity === null || anchor === undefined) {
			return;
		}
		for (const granuleValues of Object.values(metric.values)) {
			for (const time of Object.keys(granuleValues)) {
				offsets.add(bucketOffset(time, anchor, metric.granularity));
			}
		}
	});
	return Array.from(offsets).toSorted((a, b) => a - b);
};

/** The highest value shown for one metric, stacking per-bucket sums for summed metrics. */
const metricMaxValue = (metric: TrainingMetric): number => {
	let max = 0;

	if (metric.aggregate === 'Sum') {
		const totals = new Map<string, number>();
		for (const granuleValues of Object.values(metric.values)) {
			for (const [time, value] of Object.entries(granuleValues)) {
				totals.set(time, (totals.get(time) ?? 0) + value);
			}
		}
		for (const total of totals.values()) {
			max = Math.max(max, total);
		}
		return max;
	}

	for (const granuleValues of Object.values(metric.values)) {
		for (const value of Object.values(granuleValues)) {
			max = Math.max(max, value);
		}
	}
	return max;
};

/** Extract the y axis domain, taking the max value from each training metric values */
export const yDomain = (first: TrainingMetric, second: TrainingMetric): number[] => [
	0,
	Math.max(metricMaxValue(first), metricMaxValue(second))
];

/** Built-in comparison definitions, available even when a period defines no metric. */
export const defaultCompareDefinitions = (): CompareMetricDefinition[] => [
	{
		key: 'default:weekly-distance',
		label: 'Weekly distance',
		source: 'default',
		base: {
			metric: 'Distance',
			window: { granularity: 'Weekly', aggregate: 'Sum', group_by: 'SportCategory' }
		}
	},
	{
		key: 'default:weekly-active-duration',
		label: 'Weekly active duration',
		source: 'default',
		base: {
			metric: 'ActiveDuration',
			window: { granularity: 'Weekly', aggregate: 'Sum', group_by: 'SportCategory' }
		}
	},
	{
		key: 'default:weekly-elevation',
		label: 'Weekly elevation',
		source: 'default',
		base: {
			metric: 'Elevation',
			window: { granularity: 'Weekly', aggregate: 'Sum', group_by: 'SportCategory' }
		}
	}
];

/** Display label of a comparison definition, composed when it has no name. */
export const definitionLabel = (definition: CompareMetricDefinition): string => {
	if (definition.label !== null) {
		return definition.label;
	}

	const granularity = definition.base.window?.granularity;
	const aggregate = definition.base.window?.aggregate ?? 'Sum';
	const metricName =
		aggregate === 'NumberOfActivities'
			? definition.base.metric
			: definition.base.metric.toLowerCase();

	return [
		granularity === undefined ? null : granularity.toLowerCase(),
		aggregateFunctionDisplay[aggregate],
		metricName
	]
		.filter((part) => part !== null)
		.join(' ');
};
