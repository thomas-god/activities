import type { PageLoad } from './$types';
import { fetchActivityDetails, type ActivityWithTimeseries, type Timeseries } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	depends(`app:activity:${params.activity_id}`);

	return { activity: fetchActivityDetails(fetch, params.activity_id) };
};

export const prerender = false;
export const ssr = false;

export type { ActivityWithTimeseries as ActivityDetails, Timeseries };
