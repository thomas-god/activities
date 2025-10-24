import type { PageLoad } from './$types';
import { fetchTrainingMetrics } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const startDate = url.searchParams.get('start');
	const endDate = url.searchParams.get('end');

	if (startDate === null) {
		return { metrics: { noGroup: [], metircs: [] } };
	}

	const metrics = await fetchTrainingMetrics(
		fetch,
		startDate,
		endDate !== null ? endDate : undefined
	);

	return { metrics };
};

export const prerender = false;
