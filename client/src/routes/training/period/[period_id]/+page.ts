import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import {
	fetchTrainingPeriodDetails,
	type TrainingPeriodDetails,
	type TrainingPeriodActivityItem
} from '$lib/api';
import { fetchTrainingNotes, type TrainingNote } from '$lib/api/training';

export const load: PageLoad = async ({ fetch, params, depends }) => {
	const periodDetails = await fetchTrainingPeriodDetails(fetch, params.period_id);

	if (periodDetails === null) {
		redirect(307, '/');
	}

	// Fetch training notes (we'll filter them on the client side based on period dates)
	depends('app:training-notes');
	const trainingNotes = await fetchTrainingNotes(fetch);

	return { periodDetails, trainingNotes };
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem, TrainingNote };
