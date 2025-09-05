import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	let res = await fetch(`${PUBLIC_APP_URL}/api/activities`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return { activities: ActivityList.parse(await res.json()) };
	}
	return { activities: [] };
};

export const prerender = false;

const ActivityListItem = z.object({
	id: z.string(),
	sport: z.string(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true })
});

const ActivityList = z.array(ActivityListItem);
