import { PUBLIC_APP_URL } from '$env/static/public';
import * as z from 'zod';
import type { PageLoad } from '../$types';
import { goto } from '$app/navigation';

export const prerender = true;
export const ssr = false;

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const [periods] = await Promise.all([fetchTrainingPeriods(fetch)]);

	return { periods };
};

const fetchTrainingPeriods = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<TrainingPeriodList> => {
	let fetchUrl = `${PUBLIC_APP_URL}/api/training/periods`;

	const res = await fetch(fetchUrl, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});
	if (res.status === 401) {
		goto('/login');
	}
	if (res.status === 200) {
		return TrainingPeriodList.parse(await res.json());
	}
	return [];
};

const TrainingPeriodListItem = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.object({
		sports: z.array(z.string()),
		categories: z.array(z.string())
	}),
	note: z.string().nullable()
});

const TrainingPeriodList = z.array(TrainingPeriodListItem);

export type TrainingPeriodList = z.infer<typeof TrainingPeriodList>;
export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItem>;
