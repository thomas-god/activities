import type { PageLoad } from './$types';
import { fetchTrainingMetrics, fetchAllPreferences } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const startDate = url.searchParams.get('start');
	const endDate = url.searchParams.get('end');

	if (startDate === null) {
		return { metrics: { noGroup: [], metrics: [] }, preferences: [] };
	}

	const [metrics, preferences] = await Promise.all([
		fetchTrainingMetrics(fetch, startDate, endDate !== null ? endDate : undefined),
		fetchAllPreferences(fetch)
	]);

	return { metrics, preferences };
};

export const prerender = false;
