import type { PageLoad } from './$types';
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

	return {
		periodDetails: fetchTrainingPeriodDetails(fetch, params.period_id),
		trainingNotes: fetchTrainingPeriodNotes(fetch, params.period_id),
		metrics: fetchTrainingPeriodMetrics(fetch, params.period_id)
	};
};

export const prerender = false;
export const ssr = false;

export type { TrainingPeriodDetails, TrainingPeriodActivityItem, TrainingNote };
