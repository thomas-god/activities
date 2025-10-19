import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import { redirect } from '@sveltejs/kit';
import { goto } from '$app/navigation';

export const load: PageLoad = async ({ fetch, depends, params }) => {
	let res = await fetch(`${PUBLIC_APP_URL}/api/training/period/${params.period_id}`, {
		method: 'GET',
		credentials: 'include',
		mode: 'cors'
	});
	if (res.status === 401) {
		goto('/login');
	}
	if (res.status === 200) {
		return { periodDetails: TrainingPeriodDetails.parse(await res.json()) };
	}

	redirect(307, '/');
};

export const prerender = false;
export const ssr = false;

const TrainingPeriodDetails = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.array(z.string()),
	note: z.string().nullable()
});

export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetails>;
