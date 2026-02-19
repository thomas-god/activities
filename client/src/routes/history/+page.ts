import type { PageLoad } from './$types';
import { fetchActivities, fetchTrainingNotes } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	return { activities: fetchActivities(fetch), notes: fetchTrainingNotes(fetch, depends) };
};

export const prerender = false;
