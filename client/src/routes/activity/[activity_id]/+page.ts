import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import { redirect } from '@sveltejs/kit';
import { goto } from '$app/navigation';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	depends(`app:activity:${params.activity_id}`);

	let res = await fetch(`${PUBLIC_APP_URL}/api/activity/${params.activity_id}`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors'
	});
	if (res.status === 401) {
		goto('/login');
	}
	if (res.status === 200) {
		return { activity: ActivityDetails.parse(await res.json()) };
	}
	redirect(307, '/');
};

export const prerender = false;
export const ssr = false;

const ActivityDetails = z.object({
	id: z.string(),
	sport: z.string(),
	name: z.string().nullable(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true }),
	statistics: z.record(z.string(), z.number()),
	timeseries: z.object({
		time: z.array(z.number()),
		metrics: z.record(
			z.string(),
			z.object({
				unit: z.string(),
				values: z.array(z.number().nullable())
			})
		)
	})
});

export type ActivityDetails = z.infer<typeof ActivityDetails>;
export type Timeseries = ActivityDetails['timeseries'];
