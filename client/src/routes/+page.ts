import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingMetrics, fetchTrainingPeriods } from '$lib/api';
import { fetchActiveTrainingPeriods, fetchTrainingNotes } from '$lib/api/training';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const startDate = dayjs().startOf('isoWeek').subtract(3, 'weeks').toDate();
	const endDate = dayjs().add(1, 'day').endOf('day').toDate();

	return {
		activitiesWithNotes: Promise.all([
			fetchActivities(fetch, undefined, startDate, endDate),
			fetchTrainingNotes(fetch, depends, startDate, endDate)
		]),
		metrics: fetchTrainingMetrics(fetch, startDate, undefined, 'global'),
		trainingPeriods: fetchActiveTrainingPeriods(fetch, new Date())
	};
};

export const prerender = false;
