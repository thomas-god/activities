import type { PageLoad } from './$types';
import { fetchActivities } from '$lib/api';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const activities = await fetchActivities(fetch);

	return { activities };
};

export const prerender = false;
