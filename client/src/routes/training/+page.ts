import type { PageLoad } from './$types';
import * as z from 'zod';

import { PUBLIC_APP_URL } from '$env/static/public';
import dayjs from 'dayjs';

export const load: PageLoad = async ({ fetch, depends, url }) => {
	depends('app:training-metrics');

	const startDate = url.searchParams.get('start');
	if (startDate === null) {
		return { metrics: [] };
	}
	const start = encodeURIComponent(dayjs(startDate).format('YYYY-MM-DDTHH:mm:ssZ'));
	const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics?start=${start}`, {
		method: 'GET',
		mode: 'cors',
		credentials: 'include'
	});
	if (res.status === 200) {
		return { metrics: MetricsList.parse(await res.json()) };
	}
	return { metrics: [] };
};

export const prerender = false;

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
