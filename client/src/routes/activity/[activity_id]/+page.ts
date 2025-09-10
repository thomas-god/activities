import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	depends(`app:activity:${params.activity_id}`);

	let res = await fetch(`${PUBLIC_APP_URL}/api/activity/${params.activity_id}`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return { activity: ActivityDetails.parse(await res.json()) };
	}
	return { activity: undefined };
};

export const prerender = false;
export const ssr = false;

const ActivityDetails = z.object({
	id: z.string(),
	sport: z.string(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true }),
	timeseries: z.object({
		time: z.array(z.number()),
		metrics: z.record(z.string(), z.array(z.number().nullable()))
	})
});

export type ActivityDetails = z.infer<typeof ActivityDetails>;
export type Timeseries = ActivityDetails['timeseries'];
