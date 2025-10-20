import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingMetrics, fetchTrainingPeriods } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const [activities, metrics, trainingPeriods] = await Promise.all([
		fetchActivities(fetch, 10),
		fetchTrainingMetrics(fetch),
		fetchTrainingPeriods(fetch)
	]);

	return { activities, metrics, trainingPeriods };
};

export const prerender = false;
