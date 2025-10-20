import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import { dayjs } from '$lib/duration';
import { goto } from '$app/navigation';
import { SportCategories, sports } from '$lib/sport';
import { metricAggregateFunctions } from '$lib/metric';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const [activities, metrics, trainingPeriods] = await Promise.all([
		fetchActivities(fetch),
		fetchMetrics(fetch),
		fetchTrainingPeriods(fetch)
	]);

	return { activities, metrics, trainingPeriods };
};

const fetchActivities = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<ActivityList> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/activities?limit=10`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});
	if (res.status === 401) {
		goto('/login');
	}

	if (res.status === 200) {
		return ActivityList.parse(await res.json());
	}
	return [];
};

const fetchMetrics = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<MetricsList> => {
	let start = dayjs().startOf('isoWeek').subtract(3, 'week');
	let startUri = encodeURIComponent(start.format('YYYY-MM-DDTHH:mm:ssZ'));
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics?start=${startUri}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});
	if (res.status === 401) {
		goto('/login');
	}
	if (res.status === 200) {
		return MetricsList.parse(await res.json());
	}
	return [];
};

const fetchTrainingPeriods = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<TrainingPeriodList> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/periods`, {
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

export const prerender = false;

const ActivityListItem = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.string(),
	sport_category: z.enum(SportCategories).nullable(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true })
});

const ActivityList = z.array(ActivityListItem);

export type ActivityList = z.infer<typeof ActivityList>;
export type ActivityListItem = z.infer<typeof ActivityListItem>;

const MetricsListItem = z.object({
	id: z.string(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.enum(metricAggregateFunctions),
	values: z.record(z.string(), z.number())
});

const MetricsList = z.array(MetricsListItem);

export type MetricsList = z.infer<typeof MetricsList>;
export type MetricsListItem = z.infer<typeof MetricsListItem>;

const TrainingPeriodListItem = z.object({
	id: z.string(),
	start: z.string(),
	end: z.string().nullable(),
	name: z.string(),
	sports: z.object({
		sports: z.array(z.enum(sports)),
		categories: z.array(z.enum(SportCategories))
	}),
	note: z.string().nullable()
});

const TrainingPeriodList = z.array(TrainingPeriodListItem);

export type TrainingPeriodList = z.infer<typeof TrainingPeriodList>;
export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItem>;
