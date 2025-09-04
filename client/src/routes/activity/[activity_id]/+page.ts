import type { PageLoad } from './$types';

import { PUBLIC_APP_URL } from '$env/static/public';
import { ActivityListItem } from '$lib/types/activity';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	depends(`app:activity:${params.activity_id}`);

	let res = await fetch(`${PUBLIC_APP_URL}/api/activity/${params.activity_id}`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return { activity: ActivityListItem.parse(await res.json()) };
	}
	return { activity: undefined };
};

export const prerender = false;
