import { getBonkStatusLabel, type BonkStatus } from './nutrition';
import { sportCategoryDisplay, sportDisplay, type Sport, type SportCategory } from './sport';
import { workoutTypeDisplay, type WorkoutType } from './workout-type';

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
