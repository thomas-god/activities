import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import dayjs from 'dayjs';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:activities');

	const [activities, metrics] = await Promise.all([fetchActivities(fetch), fetchMetrics(fetch)]);

	return { activities, metrics };
};

const fetchActivities = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<ActivityList> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/activities`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return ActivityList.parse(await res.json());
	}
	return [];
};

const fetchMetrics = async (
	fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
): Promise<MetricsList> => {
	let now = dayjs();
	let start = encodeURIComponent(now.subtract(1, 'month').format('YYYY-MM-DDTHH:mm:ssZ'));
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics?start=${start}`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return MetricsList.parse(await res.json());
	}
	return [];
};

export const prerender = false;

const ActivityListItem = z.object({
	id: z.string(),
	name: z.string().nullable(),
	sport: z.string(),
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
	aggregate: z.string(),
	values: z.record(z.string(), z.number())
});

const MetricsList = z.array(MetricsListItem);

export type MetricsList = z.infer<typeof MetricsList>;
export type MetricsListItem = z.infer<typeof MetricsListItem>;
