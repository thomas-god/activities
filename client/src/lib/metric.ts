import { getBonkStatusLabel, type BonkStatus } from './nutrition';
import { sportCategoryDisplay, sportDisplay, type Sport, type SportCategory } from './sport';
import { workoutTypeDisplay, type WorkoutType } from './workout-type';

export const activityStatistics = [
	'Calories',
	'Elevation',
	'Distance',
	'Duration',
	'NormalizedPower'
] as const;
export type ActivityStatistic = (typeof activityStatistics)[number];

export const metricAggregateFunctions = [
	'Min',
	'Max',
	'Average',
	'Sum',
	'NumberOfActivities'
] as const;
export type MetricAggregateFunction = (typeof metricAggregateFunctions)[number];

export const aggregateFunctionDisplay: Record<MetricAggregateFunction, string> = {
	Average: 'activity average',
	Max: 'maximum',
	Min: 'minimum',
	Sum: 'total',
	NumberOfActivities: 'number of activities'
};

export const groupByClauses = [
	'Sport',
	'SportCategory',
	'WorkoutType',
	'RpeRange',
	'Bonked'
] as const;
export type GroupByClause = (typeof groupByClauses)[number];

export const groupByClauseDisplay = (clause: GroupByClause): string => {
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

export const displayGroupName = (group: string, groupBy: GroupByClause | null): string => {
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
