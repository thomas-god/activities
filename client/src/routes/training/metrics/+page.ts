import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import dayjs from 'dayjs';
import { goto } from '$app/navigation';

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const [metrics, periods] = await Promise.all([
		fetchMetrics(fetch, url),
		fetchTrainingPeriods(fetch)
	]);

	return { metrics, periods };
};

const fetchMetrics = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
	url: URL
): Promise<MetricsList> => {
	const startDate = url.searchParams.get('start');
	if (startDate === null) {
		return [];
	}
	const endDate = url.searchParams.get('end');
	let fetchUrl = `${PUBLIC_APP_URL}/api/training/metrics?start=${encodeURIComponent(dayjs(startDate).format('YYYY-MM-DDTHH:mm:ssZ'))}`;
	if (endDate !== null) {
		fetchUrl =
			fetchUrl + `&end=${encodeURIComponent(dayjs(endDate).format('YYYY-MM-DDTHH:mm:ssZ'))}`;
	}
	const res = await fetch(fetchUrl, {
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

export const prerender = false;

const MetricsListItem = z.object({
	id: z.string(),
	metric: z.string(),
	unit: z.string(),
	granularity: z.string(),
	aggregate: z.string(),
	sports: z.array(z.string()),
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
	sports: z.array(z.string()),
	note: z.string().nullable()
});

const TrainingPeriodList = z.array(TrainingPeriodListItem);

export type TrainingPeriodList = z.infer<typeof TrainingPeriodList>;
export type TrainingPeriodListItem = z.infer<typeof TrainingPeriodListItem>;
