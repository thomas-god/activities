import { getBonkStatusLabel, type BonkStatus } from './nutrition';
import { sportCategoryDisplay, sportDisplay, type Sport, type SportCategory } from './sport';
import { workoutTypeDisplay, type WorkoutType } from './workout-type';
import { dayjs, now as dayjsNow } from './duration';
import type {
	TrainingMetric,
	TrainingMetricBasePayload,
	PreviewTrainingMetricPayload,
	TrainingPeriodDetails
} from '$lib/api';
import { asOption, isNone, none, type Option } from './Options';

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
	'Feedback',
	'Weight',
	'Nutrition',
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
	groupBy: Option<TrainingMetricGroupByClause>
): string => {
	if (group === 'Other') {
		return 'Other';
	}

	if (isNone(groupBy)) {
		return group;
	}

	switch (groupBy.value) {
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
	source: string;
	base: TrainingMetricBasePayload;
};

/**
 * Identity of a metric definition, used to deduplicate metrics that exist in
 * both periods. Deliberately excludes the display name and target.
 */
export const metricDefinitionKey = (metric: TrainingMetric): string =>
	JSON.stringify({
		source: metric.source,
		granularity: metric.granularity,
		aggregate: metric.aggregate
	});

export const definitionGroupBy = (
	definition: CompareMetricDefinition
): Option<TrainingMetricGroupByClause> => {
	if (
		definition.base.source.type === 'activity' &&
		definition.base.source.metric.group_by !== undefined
	) {
		return asOption(definition.base.source.metric.group_by);
	} else {
		return none();
	}
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
	const payload: TrainingMetricBasePayload = { source: metric.source };

	if (metric.granularity !== null && metric.aggregate !== null) {
		payload.window = {
			granularity: metric.granularity,
			aggregate: metric.aggregate
		};
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
	period: Pick<TrainingPeriodDetails, 'start' | 'end' | 'sports'>,
	now = dayjsNow
): PreviewTrainingMetricPayload => {
	return {
		...definition.base,
		...periodMetricRange(period, now)
	};
};

/**
 * The date range over which a metric is computed for a period. The range end
 * is extended by one day so the period's last day is included (now when the
 * period is ongoing).
 */
export const periodMetricRange = (
	period: Pick<TrainingPeriodDetails, 'start' | 'end'>,
	now = dayjsNow
): { start: string; end: string } => ({
	start: dayjs(period.start).format('YYYY-MM-DD'),
	end: (period.end === null ? now() : dayjs(period.end)).add(1, 'day').format('YYYY-MM-DD')
});

/** Built-in comparison definitions, available even when a period defines no metric. */
export const defaultCompareDefinitions = (): CompareMetricDefinition[] => [
	{
		key: 'default:weekly-distance',
		label: 'Weekly distance',
		source: 'default',
		base: {
			source: {
				type: 'activity',
				metric: {
					metric: 'Distance',
					group_by: 'SportCategory',
					filters: {
						bonked: null,
						rpes: null,
						sports: null,
						workout_types: null
					}
				}
			},
			window: { granularity: 'Weekly', aggregate: 'Sum' },
			summary: { average: { include_zeros: false } }
		}
	},
	{
		key: 'default:weekly-active-duration',
		label: 'Weekly duration',
		source: 'default',
		base: {
			source: {
				type: 'activity',
				metric: {
					metric: 'ActiveDuration',
					group_by: 'SportCategory',
					filters: {
						bonked: null,
						rpes: null,
						sports: null,
						workout_types: null
					}
				}
			},
			window: { granularity: 'Weekly', aggregate: 'Sum' },
			summary: { average: { include_zeros: false } }
		}
	},
	{
		key: 'default:weekly-elevation',
		label: 'Weekly elevation',
		source: 'default',
		base: {
			source: {
				type: 'activity',
				metric: {
					metric: 'Elevation',
					group_by: 'SportCategory',
					filters: {
						bonked: null,
						rpes: null,
						sports: null,
						workout_types: null
					}
				}
			},
			window: { granularity: 'Weekly', aggregate: 'Sum' },
			summary: { average: { include_zeros: false } }
		}
	},

	{
		key: 'default:weekly-calories',
		label: 'Weekly calories',
		source: 'default',
		base: {
			source: {
				type: 'activity',
				metric: {
					metric: 'Calories',
					group_by: 'SportCategory',
					filters: {
						bonked: null,
						rpes: null,
						sports: null,
						workout_types: null
					}
				}
			},
			window: { granularity: 'Weekly', aggregate: 'Sum' },
			summary: { average: { include_zeros: false } }
		}
	}
];

const metricAsString = (definition: CompareMetricDefinition): string => {
	if (definition.base.source.type === 'activity') {
		return definition.base.source.metric.metric.toLocaleLowerCase();
	} else {
		return definition.base.source.metric.toLocaleLowerCase();
	}
};

/** Display label of a comparison definition, composed when it has no name. */
export const definitionLabel = (definition: CompareMetricDefinition): string => {
	if (definition.label !== null) {
		return definition.label;
	}

	const granularity = definition.base.window?.granularity;
	const aggregate = definition.base.window?.aggregate ?? 'Sum';
	const metricName =
		aggregate === 'NumberOfActivities' ? definition.base.source.metric : metricAsString(definition);

	return [
		granularity === undefined ? null : granularity.toLowerCase(),
		aggregateFunctionDisplay[aggregate],
		metricName
	]
		.filter((part) => part !== null)
		.join(' ');
};
