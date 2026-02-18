import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingMetrics, fetchTrainingPeriods } from '$lib/api';
import { fetchTrainingNotes } from '$lib/api/training';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const startDate = dayjs().startOf('isoWeek').subtract(3, 'weeks').toDate();
	const endDate = dayjs().add(1, 'day').endOf('day').toDate();

	const [activities, metrics, trainingPeriods, trainingNotes] = await Promise.all([
		fetchActivities(fetch, undefined, startDate, endDate),
		fetchTrainingMetrics(fetch, startDate, undefined, 'global'),
		fetchTrainingPeriods(fetch),
		fetchTrainingNotes(fetch, depends, startDate, endDate)
	]);

	return {
		activities,
		metrics,
		trainingPeriods,
		trainingNotes
	};
};

export const prerender = false;
