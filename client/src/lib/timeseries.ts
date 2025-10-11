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

	const max = values.values.reduce(
		(max, curr) => (curr === null ? max : curr > max ? curr : max),
		Number.NEGATIVE_INFINITY
	);

	if (max === Number.NEGATIVE_INFINITY) {
		return undefined;
	}

	return max;
};
