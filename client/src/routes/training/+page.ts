import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import dayjs from 'dayjs';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:training-metrics');

	let now = dayjs();
	let start = encodeURIComponent(now.subtract(1, 'month').format('YYYY-MM-DDTHH:mm:ssZ'));
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics?start=${start}`, {
		method: 'GET'
	});
	if (res.status === 200) {
		return { metrics: MetricsList.parse(await res.json()) };
	}
	return { metrics: [] };
};

export const prerender = false;

const ActivityListItem = z.object({
	id: z.string(),
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
	granularity: z.string(),
	aggregate: z.string(),
	values: z.record(z.string(), z.number())
});

const MetricsList = z.array(MetricsListItem);

export type MetricsList = z.infer<typeof MetricsList>;
export type MetricsListItem = z.infer<typeof MetricsListItem>;
