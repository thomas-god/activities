import type { PageLoad } from './$types';

import { PUBLIC_APP_URL } from '$env/static/public';
import * as z from 'zod';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	let res = await fetch(`${PUBLIC_APP_URL}/api/activities`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return { activities: Activities.parse(await res.json()) };
	}
	return { activities: [] };
};

const Activities = z.array(
	z.object({
		id: z.string(),
		sport: z.string(),
		duration: z.number().nullable(),
		calories: z.number().nullable()
	})
);

type Activities = z.infer<typeof Activities>;
