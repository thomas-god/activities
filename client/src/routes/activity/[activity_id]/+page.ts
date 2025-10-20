import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { fetchActivityDetails, type ActivityDetails } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	depends(`app:activity:${params.activity_id}`);

	const activity = await fetchActivityDetails(fetch, params.activity_id);

	if (activity === null) {
		redirect(307, '/');
	}

	return { activity };
};

export const prerender = false;
export const ssr = false;

export type { ActivityDetails };
export type Timeseries = ActivityDetails['timeseries'];
