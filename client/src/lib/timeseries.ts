import type { Timeseries } from '../routes/activity/[activity_id]/+page';

export const convertTimeseriesToActiveTime = (
	timeseries: Timeseries
): { time: number[]; metrics: Timeseries['metrics'] } => {
	const active_metrics: Record<
		string,
		{
			unit: string;
			values: (number | null)[];
		}
	> = {};

	for (const [metric, values] of Object.entries(timeseries.metrics)) {
		const active_values = values.values.filter(
			(_value, time_index) => timeseries.active_time.at(time_index) !== null
		);

		active_metrics[metric] = {
			unit: values.unit,
			values: active_values
		};
	}

	const active_time = timeseries.active_time.filter((val) => val !== null);

	return {
		time: active_time,
		metrics: active_metrics
	};
};
