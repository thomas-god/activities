import type { PageLoad } from './$types';
import {
	fetchActivities,
	fetchActivityDefaultMetrics,
	fetchActivityListSummary,
	fetchTrainingNotes
} from '$lib/api';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	return {
		activities: fetchActivities(fetch),
		notes: fetchTrainingNotes(fetch, depends),
		defaultMetrics: fetchActivityDefaultMetrics(fetch),
		activityListSummary: fetchActivityListSummary(fetch)
	};
};

export const prerender = false;
