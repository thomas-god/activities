import { describe, expect, it } from 'vitest';
import type { Timeseries } from '../routes/activity/[activity_id]/+page.ts';
import {
	convertTimeseriesToActiveTime,
	timeseriesAvg,
	timeseriesMaximum,
	timeseriesQuarticAvg
} from './timeseries';

describe('Finding the average value of a timeseries', () => {
	it('Should return undefined when the timeseries is not found', () => {
		const timeseries: Timeseries['metrics'] = {};

		const res = timeseriesAvg(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});

	it('Should return the average', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [1, 2, 3] } };

		const res = timeseriesAvg(timeseries, 'Calories');

		expect(res).toEqual(2);
	});

	it('Should ignore null values', () => {
		const timeseries: Timeseries['metrics'] = {
			Calories: { unit: 'kcal', values: [1, 2, 3, null, null] }
		};

		const res = timeseriesAvg(timeseries, 'Calories');

		expect(res).toEqual(2);
	});

	it('Should return undefined if there are no non-null values', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [null, null] } };

		const res = timeseriesAvg(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});
});

describe('Finding the quartic average value of a timeseries', () => {
	it('Should return undefined when the timeseries is not found', () => {
		const timeseries: Timeseries['metrics'] = {};

		const res = timeseriesQuarticAvg(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});

	it('Should return the average', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [1, 2, 3] } };

		const res = timeseriesQuarticAvg(timeseries, 'Calories');

		expect(res).toEqual(Math.pow((1 + Math.pow(2, 4) + Math.pow(3, 4)) / 3, 1 / 4));
	});

	it('Should ignore null values', () => {
		const timeseries: Timeseries['metrics'] = {
			Calories: { unit: 'kcal', values: [1, 2, 3, null, null] }
		};

		const res = timeseriesQuarticAvg(timeseries, 'Calories');

		expect(res).toEqual(Math.pow((1 + Math.pow(2, 4) + Math.pow(3, 4)) / 3, 1 / 4));
	});

	it('Should return undefined if there are no non-null values', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [null, null] } };

		const res = timeseriesQuarticAvg(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});
});

describe('Finding the maximum value of a timeseries', () => {
	it('Should return undefined when the timeseries is not found', () => {
		const timeseries: Timeseries['metrics'] = {};

		const res = timeseriesMaximum(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});

	it('Should return the average', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [1, 2, 3] } };

		const res = timeseriesMaximum(timeseries, 'Calories');

		expect(res).toEqual(3);
	});

	it('Should ignore null values', () => {
		const timeseries: Timeseries['metrics'] = {
			Calories: { unit: 'kcal', values: [1, 2, 3, null, null] }
		};

		const res = timeseriesMaximum(timeseries, 'Calories');

		expect(res).toEqual(3);
	});

	it('Should return undefined if there are no non-null values', () => {
		const timeseries: Timeseries['metrics'] = { Calories: { unit: 'kcal', values: [null, null] } };

		const res = timeseriesMaximum(timeseries, 'Calories');

		expect(res).toBeUndefined();
	});
});

describe('Converting a time series from absolute time to active time', () => {
	it('Should use the active_time data', () => {
		const timeseries: Timeseries = {
			time: [1, 2, 3],
			active_time: [1, null, 2],
			metrics: {
				power: {
					unit: 'W',
					values: [100, 120, 130]
				}
			}
		};

		let activeTimeseries = convertTimeseriesToActiveTime(timeseries);

		expect(activeTimeseries.time).toEqual([1, 2]);
		expect(activeTimeseries.metrics.power.values).toEqual([100, 130]);
	});
});
