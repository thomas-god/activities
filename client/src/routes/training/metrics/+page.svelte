<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import DateRangeSelector from '../../../organisms/DateRangeSelector.svelte';
	import TrainingMetricsChart from '../../../organisms/TrainingMetricsChart.svelte';
	import { dayjs } from '$lib/duration';
	import type { PageProps } from './$types';
	import { aggregateFunctionDisplay } from '$lib/metric';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	const formatMetricTitle = (metric: (typeof data.metrics)[number]): string => {
		return `${capitalize(metric.granularity.toLowerCase())} ${aggregateFunctionDisplay[metric.aggregate]}  ${metric.metric.toLowerCase()}  [${metric.sports.length > 0 ? metric.sports.join(', ') : 'All sports'}]`;
	};
	let metricsProps = $derived.by(() => {
		let metrics = [];
		for (let i = 0; i < data.metrics.length; i++) {
			let metric = data.metrics.at(i);
			if (metric === undefined) {
				continue;
			}
			let values = [];
			for (const dt in metric.values) {
				values.push({ time: dt, value: metric.values[dt] });
			}

			metrics.push({
				values: values,
				title: formatMetricTitle(metric),
				unit: metric.unit,
				id: metric.id,
				granularity: metric.granularity
			});
		}
		return metrics;
	});

	const deleteMetricCallback = async (metric: string): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (res.status === 401) {
			goto('/login');
		}
		invalidate('app:training-metrics');
	};

	$effect(() => {
		// Redirect if no start parameter
		const startDate = page.url.searchParams.get('start');
		if (startDate === null) {
			const now = dayjs();
			const start = encodeURIComponent(now.subtract(1, 'month').format('YYYY-MM-DD'));
			goto(`${page.url.toString()}?start=${start}`);
		}
	});

	const datesUpdateCallback = (newDates: { start: string; end: string }) => {
		let url = page.url.pathname.toString();
		url += `?start=${encodeURIComponent(dayjs(newDates.start).format('YYYY-MM-DD'))}`;
		if (newDates.end !== dayjs().format('YYYY-MM-DD')) {
			// For convenience, don't add end date if it's today
			url += `&end=${encodeURIComponent(dayjs(newDates.end).format('YYYY-MM-DD'))}`;
		}
		goto(url);
	};
</script>

<div class="mx-auto flex flex-col gap-4">
	<DateRangeSelector {dates} {datesUpdateCallback} periods={data.periods} />

	{#each metricsProps as metric}
		<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 shadow-md">
			<div class="relative p-4 text-center">
				<div>
					{metric.title}
				</div>
				<button
					class="btn bg-base-100 hover:outline-base-300 absolute bottom-[8px] right-4 border-0 p-0 shadow-none hover:outline-2"
					onclick={() => deleteMetricCallback(metric.id)}>🗑️</button
				>
			</div>
			<TrainingMetricsChart
				height={250}
				width={chartWidth}
				values={metric.values}
				unit={metric.unit}
				granularity={metric.granularity}
				format={metric.unit === 's' ? 'duration' : 'number'}
			/>
		</div>
	{/each}
</div>
