import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const activities = await fetchActivities(fetch);

	return { activities };
};

const fetchActivities = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<ActivityList> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/activities`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});
	if (res.status === 401) {
		goto('/login');
	}

	if (res.status === 200) {
		return ActivityList.parse(await res.json());
	}
	return [];
};

export const prerender = false;

const ActivityListItem = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.string(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true })
});

const ActivityList = z.array(ActivityListItem);

export type ActivityList = z.infer<typeof ActivityList>;
export type ActivityListItem = z.infer<typeof ActivityListItem>;
