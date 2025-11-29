import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingMetrics, fetchTrainingPeriods } from '$lib/api';
import { fetchTrainingNotes } from '$lib/api/training';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const startDate = dayjs().startOf('isoWeek').subtract(3, 'weeks').toDate();

	const [activities, metrics, trainingPeriods, trainingNotes] = await Promise.all([
		fetchActivities(fetch, 10),
		fetchTrainingMetrics(fetch, startDate, undefined, 'global'),
		fetchTrainingPeriods(fetch),
		fetchTrainingNotes(fetch, depends)
	]);

	// Filter notes to only include those after (inclusive) the oldest activity date
	const filteredNotes =
		activities.length > 0
			? trainingNotes.filter((note) => {
					const oldestActivityDate = dayjs(activities[activities.length - 1].start_time).startOf(
						'day'
					);
					const noteDate = dayjs(note.date).startOf('day');
					return noteDate.isAfter(oldestActivityDate) || noteDate.isSame(oldestActivityDate);
				})
			: trainingNotes;

	return {
		activities,
		metrics,
		trainingPeriods,
		trainingNotes: filteredNotes
	};
};

export const prerender = false;
