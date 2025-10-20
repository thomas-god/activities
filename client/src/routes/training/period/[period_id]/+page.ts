import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import {
	fetchTrainingPeriodDetails,
	type TrainingPeriodDetails,
	type TrainingPeriodActivityItem
} from '$lib/api';

export const load: PageLoad = async ({ fetch, params }) => {
	const periodDetails = await fetchTrainingPeriodDetails(fetch, params.period_id);

	if (periodDetails === null) {
		redirect(307, '/');
	}

	return { periodDetails };
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem };
