<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import TrainingMetricsOptions from '$ui/training_metrics/TrainingMetricsOptions.svelte';
	import { dayjs } from '$lib/duration';
	import TrainingMetricTitle from '$ui/training_metrics/TrainingMetricTitle.svelte';
	import { fetchTrainingMetrics, fetchTrainingPeriods, type TrainingMetricList } from '$lib/api';
	import { isSome, some, type Option } from '$lib/Options';
	import NavbarMetrics from '$ui/navigation/NavbarMetrics.svelte';
	import TrainingMetricChart from '$ui/training_metrics/TrainingMetricChart.svelte';

	let chartWidths: number[] = $state([]);

	let dates = $derived({
		start: page.url.searchParams.get('start') || dayjs().subtract(1, 'month').format('YYYY-MM-DD'),
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	const generateMetricsPromise = () =>
		some(fetchTrainingMetrics(fetch, dates.start, dates.end, 'global'));
	const setMetricsPromise = () => (metricsPromise = generateMetricsPromise());
	let metricsPromise: Option<Promise<TrainingMetricList>> = $derived(generateMetricsPromise());

	let periodsPromise = $state(some(fetchTrainingPeriods(fetch)));

	const datesUpdateCallback = (newDates: { start: string; end: string }) => {
		let url = page.url.pathname.toString();
		url += `?start=${encodeURIComponent(dayjs(newDates.start).format('YYYY-MM-DD'))}`;
		if (newDates.end !== dayjs().format('YYYY-MM-DD')) {
			// For convenience, don't add end date if it's today
			url += `&end=${encodeURIComponent(dayjs(newDates.end).format('YYYY-MM-DD'))}`;
		}
		/* eslint-disable svelte/no-navigation-without-resolve */
		goto(url);
	};
</script>

<NavbarMetrics invalidateTrainingMetrics={setMetricsPromise} />

<div class="mx-auto flex flex-col gap-4 pt-5">
	<TrainingMetricsOptions
		{dates}
		{datesUpdateCallback}
		{periodsPromise}
		metricsOrderingScope={{ type: 'global' }}
		{metricsPromise}
		onMetricsReordered={setMetricsPromise}
	/>
	{#if isSome(metricsPromise)}
		{#await metricsPromise.value}
			<div class="flex w-full flex-col items-center p-4 pt-6">
				<div class="loading loading-bars"></div>
			</div>
		{:then metrics}
			<div class="@container">
				<div class="grid grid-cols-1 gap-4 @min-[900px]:grid-cols-2">
					{#each metrics as metric, idx (metric.id)}
						<div bind:clientWidth={chartWidths[idx]} class="rounded-box bg-base-100 pb-3 shadow-md">
							<div class="relative p-4 text-center">
								<TrainingMetricTitle {metric} onUpdate={setMetricsPromise} />
							</div>
							<TrainingMetricChart
								{metric}
								width={chartWidths[idx] ?? 300}
								timeDomain={some(dates)}
							/>
						</div>
					{/each}
				</div>
			</div>
		{/await}
	{/if}
</div>
