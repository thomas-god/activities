import type { PageLoad } from './$types';

import { PUBLIC_APP_URL } from '$env/static/public';
import { ActivityList } from '$lib/types/activity';

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