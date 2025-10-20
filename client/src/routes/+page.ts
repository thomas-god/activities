import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingMetrics, fetchTrainingPeriods } from '$lib/api';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const startDate = dayjs().startOf('isoWeek').subtract(3, 'weeks').toDate();

	const [activities, metrics, trainingPeriods] = await Promise.all([
		fetchActivities(fetch, 10),
		fetchTrainingMetrics(fetch, startDate),
		fetchTrainingPeriods(fetch)
	]);

	return { activities, metrics, trainingPeriods };
};

export const prerender = false;
