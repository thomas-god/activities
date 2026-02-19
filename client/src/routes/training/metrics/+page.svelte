<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import TrainingMetricsOptions from '$components/organisms/TrainingMetricsOptions.svelte';
	import { dayjs } from '$lib/duration';
	import type { PageProps } from './$types';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import TrainingMetricMenu from '$components/molecules/TrainingMetricMenu.svelte';
	import { metricValuesDisplayFormat } from '$lib/metric';
	import { groupMetricValues, metricScope } from '$lib/api';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

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

<div class="mx-auto flex flex-col gap-4 pt-5">
	{#await data.metrics}
		<div class="flex w-full flex-col items-center p-4 pt-6">
			<div class="loading loading-bars"></div>
		</div>
	{:then metrics}
		<TrainingMetricsOptions
			{dates}
			{datesUpdateCallback}
			periods={data.periods}
			metricsOrderingScope={{ type: 'global' }}
			{metrics}
			onMetricsReordered={() => invalidate('app:training-metrics')}
		/>

		{#each metrics as metric}
			<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 pb-3 shadow-md">
				<div class="relative p-4 text-center">
					<TrainingMetricTitle
						name={metric.name}
						granularity={metric.granularity}
						aggregate={metric.aggregate}
						metric={metric.metric}
						sports={metric.sports}
						groupBy={metric.group_by}
					/>
					<div class="absolute right-4 bottom-[8px]">
						<!-- Action menu dropdown -->
						<TrainingMetricMenu
							name={metric.name}
							id={metric.id}
							scope={metricScope(metric)}
							onUpdate={() => invalidate('app:training-metrics')}
							onDelete={() => invalidate('app:training-metrics')}
						/>
					</div>
				</div>
				<TrainingMetricsChartStacked
					height={250}
					width={chartWidth}
					values={groupMetricValues(metric)}
					unit={metric.unit}
					granularity={metric.granularity}
					format={metricValuesDisplayFormat(metric)}
					showGroup={metric.group_by !== null}
					groupBy={metric.group_by}
				/>
			</div>
		{/each}
	{/await}
</div>
