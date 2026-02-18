import type { PageLoad } from './$types';
import { fetchActivities } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	return { activities: fetchActivities(fetch) };
};

export const prerender = false;
