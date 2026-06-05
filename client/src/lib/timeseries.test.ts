import { describe, expect, it } from 'vitest';
import type { Timeseries } from '../routes/activity/[activity_id]/+page.ts';
import { convertTimeseriesToActiveTime } from './timeseries';

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
			},
			laps: []
		};

		let activeTimeseries = convertTimeseriesToActiveTime(timeseries);

		expect(activeTimeseries.time).toEqual([1, 2]);
		expect(activeTimeseries.metrics.power.values).toEqual([100, 130]);
	});
});
