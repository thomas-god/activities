import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import {
	fetchTrainingPeriodDetails,
	type TrainingPeriodDetails,
	type TrainingPeriodActivityItem
} from '$lib/api';
import { fetchTrainingNotes, fetchTrainingMetrics, type TrainingNote } from '$lib/api/training';
import { dayjs } from '$lib/duration';

export const load: PageLoad = async ({ fetch, params, depends }) => {
	const periodDetails = await fetchTrainingPeriodDetails(fetch, params.period_id);

	if (periodDetails === null) {
		redirect(307, '/');
	}

	// Fetch training notes (we'll filter them on the client side based on period dates)
	depends('app:training-notes');
	const trainingNotes = await fetchTrainingNotes(fetch, depends);

	// Fetch training metrics for the period date range
	const startDate = dayjs(periodDetails.start).toDate();
	const endDate = periodDetails.end ? dayjs(periodDetails.end).toDate() : new Date();
	const metrics = await fetchTrainingMetrics(fetch, startDate, endDate);

	return { periodDetails, trainingNotes, metrics };
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem, TrainingNote };
