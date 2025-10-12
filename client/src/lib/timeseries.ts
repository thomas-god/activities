import type { Timeseries } from '../routes/activity/[activity_id]/+page';

export const timeseriesAvg = (
	metrics: Timeseries['metrics'],
	metric: string
): number | undefined => {
	let res = Object.entries(metrics).find(([_metric, _values]) => _metric === metric);

	if (res === undefined) {
		return undefined;
	}

	const [_, values] = res;

	let { sum, n } = values.values.reduce(
		(acc, curr) => (curr === null ? acc : { sum: acc.sum + curr, n: acc.n + 1 }),
		{ sum: 0, n: 0 }
	);

	if (n === 0) {
		return undefined;
	}

	return sum / n;
};

export const timeseriesQuarticAvg = (
	metrics: Timeseries['metrics'],
	metric: string
): number | undefined => {
	let res = Object.entries(metrics).find(([_metric, _values]) => _metric === metric);

	if (res === undefined) {
		return undefined;
	}

	const [_, values] = res;

	let { sum, n } = values.values.reduce(
		(acc, curr) => (curr === null ? acc : { sum: acc.sum + Math.pow(curr, 4), n: acc.n + 1 }),
		{ sum: 0, n: 0 }
	);

	if (n === 0) {
		return undefined;
	}

	return Math.pow(sum / n, 1 / 4);
};

export const timeseriesMaximum = (
	metrics: Timeseries['metrics'],
	metric: string
): number | undefined => {
	let res = Object.entries(metrics).find(([_metric, _values]) => _metric === metric);

	if (res === undefined) {
		return undefined;
	}

	const [_, values] = res;

	const max = values.values.reduce<number>(
		(max, curr) => (curr === null ? max : curr > max ? curr : max),
		Number.NEGATIVE_INFINITY
	);

	if (max === Number.NEGATIVE_INFINITY) {
		return undefined;
	}

	return max as number;
};

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
