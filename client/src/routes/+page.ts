import type { PageLoad } from './$types';
import {
	fetchActivities,
	fetchTrainingMetrics,
	fetchTrainingPeriods,
	fetchAllPreferences
} from '$lib/api';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const startDate = dayjs().startOf('isoWeek').subtract(3, 'weeks').toDate();

	const [activities, metrics, trainingPeriods, preferences] = await Promise.all([
		fetchActivities(fetch, 10),
		fetchTrainingMetrics(fetch, startDate),
		fetchTrainingPeriods(fetch),
		fetchAllPreferences(fetch)
	]);

	return { activities, metrics, trainingPeriods, preferences };
};

export const prerender = false;
