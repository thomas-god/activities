import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import { redirect } from '@sveltejs/kit';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';

export const load: PageLoad = async ({ fetch, params }) => {
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

const ActivityItem = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.string(),
	sport_category: z.enum(SportCategories).nullable(),
	duration: z
		.number()
		.nullable()
		.transform((val) => val ?? 0), // Transform null to 0 for compatibility
	start_time: z.string().transform((val) => {
		// The backend returns RFC3339 format, ensure it's compatible with ISO datetime
		return val;
	})
});

const TrainingPeriodDetails = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.object({
		sports: z.array(z.enum(sports)),
		categories: z.array(z.enum(SportCategories))
	}),
	note: z.string().nullable(),
	activities: z.array(ActivityItem)
});

export type TrainingPeriodDetails = z.infer<typeof TrainingPeriodDetails>;
export type ActivityItem = z.infer<typeof ActivityItem>;
