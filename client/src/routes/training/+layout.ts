import type { PageLoad } from '../$types';
import { fetchTrainingPeriods } from '$lib/api';

export const prerender = true;
export const ssr = false;

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:training-metrics');

	const periods = await fetchTrainingPeriods(fetch);

	return { periods };
};
