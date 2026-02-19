import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import {
	fetchTrainingPeriodDetails,
	type TrainingPeriodDetails,
	type TrainingPeriodActivityItem
} from '$lib/api';
import {
	fetchTrainingPeriodNotes,
	fetchTrainingPeriodMetrics,
	type TrainingNote
} from '$lib/api/training';

export const load: PageLoad = async ({ fetch, params, depends }) => {
	depends(`app:training-period:${params.period_id}`);
	const periodDetails = await fetchTrainingPeriodDetails(fetch, params.period_id);

	if (periodDetails === null) {
		redirect(307, '/');
	}

	depends('app:training-notes');
	const [trainingNotes, metrics] = await Promise.all([
		fetchTrainingPeriodNotes(fetch, params.period_id),
		fetchTrainingPeriodMetrics(fetch, params.period_id)
	]);

	return { periodDetails, trainingNotes, metrics };
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem, TrainingNote };
