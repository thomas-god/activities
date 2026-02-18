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
	depends(`app:training-period:${params.period_id}`);
	const periodDetails = await fetchTrainingPeriodDetails(fetch, params.period_id);

	if (periodDetails === null) {
		redirect(307, '/');
	}

	// Fetch training notes and metrics for the same period date range
	depends('app:training-notes');
	const startDate = dayjs(periodDetails.start).toDate();
	const endDate = periodDetails.end ? dayjs(periodDetails.end).toDate() : new Date();

	const [trainingNotes, metrics] = await Promise.all([
		// TODO: we should be able to fetch notes and metrics by period_id, without the need for an explicit
		fetchTrainingNotes(fetch, depends, startDate, endDate),
		fetchTrainingMetrics(fetch, startDate, endDate, {
			period: params.period_id
		})
	]);

	return { periodDetails, trainingNotes, metrics };
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem, TrainingNote };
