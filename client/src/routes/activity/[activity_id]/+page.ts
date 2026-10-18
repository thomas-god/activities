import type { PageLoad } from './$types';
import {
	fetchActivityDetails,
	type ActivityWithTimeseries,
	type Timeseries
} from '#lib/api/index.js';

export const load: PageLoad = async ({ fetch, depends, params, url }) => {
	depends(`app:activity:${params.activity_id}`);

	const periodId = url.searchParams.get('period_id') ?? undefined;

	return { activity: fetchActivityDetails(fetch, params.activity_id, { periodId }) };
};

export const prerender = false;
export const ssr = false;

export type { ActivityWithTimeseries as ActivityDetails, Timeseries };
