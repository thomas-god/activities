export const activityStatistics = [
	'Calories',
	'Elevation',
	'Distance',
	'Duration',
	'NormalizedPower'
] as const;
export type ActivityStatistic = (typeof activityStatistics)[number];

export const metricAggregateFunctions = ['Min', 'Max', 'Average', 'Sum'] as const;
export type MetricAggregateFunction = (typeof metricAggregateFunctions)[number];

export const aggregateFunctionDisplay: Record<MetricAggregateFunction, string> = {
	Average: 'activity average',
	Max: 'maximum',
	Min: 'minimum',
	Sum: 'total'
};
