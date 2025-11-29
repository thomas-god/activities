import type { PageLoad } from './$types';
import { fetchTrainingMetrics, } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const startDate = url.searchParams.get('start');
	const endDate = url.searchParams.get('end');

	if (startDate === null) {
		return { metrics: [],  };
	}

	const [metrics, ] = await Promise.all([
		fetchTrainingMetrics(fetch, startDate, endDate !== null ? endDate : undefined, 'global'),
	]);

	return { metrics,  };
};

export const prerender = false;
