<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import TrainingMetricsOptions from '$components/organisms/TrainingMetricsOptions.svelte';
	import { dayjs } from '$lib/duration';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import { metricValuesDisplayFormat } from '$lib/trainingMetric';
	import {
		fetchTrainingMetrics,
		fetchTrainingPeriods,
		groupMetricValues,
		type TrainingMetricList
	} from '$lib/api';
	import { isSome, none, some, type Option } from '$lib/Options';
	import TrainingMetricsChartLine from '$components/organisms/TrainingMetricsChartLine.svelte';
	import NavbarMetrics from '$components/organisms/navigation/NavbarMetrics.svelte';

	let chartWidth: number = $state(0);

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
			{#each metrics as metric (metric.id)}
				<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 pb-3 shadow-md">
					<div class="relative p-4 text-center">
						<TrainingMetricTitle {metric} onUpdate={setMetricsPromise} />
					</div>
					{#if groupMetricValues(metric).length > 0}
						{#if metric.granularity !== null}
							<TrainingMetricsChartStacked
								height={250}
								width={chartWidth}
								values={groupMetricValues(metric)}
								unit={metric.unit}
								granularity={metric.granularity}
								format={metricValuesDisplayFormat(metric)}
								showGroup={metric.group_by !== null}
								groupBy={metric.group_by}
								stacked={metric.aggregate === 'Sum' || metric.aggregate === 'NumberOfActivities'}
								average={'average' in metric.summary ? some(metric.summary.average) : none()}
							/>
						{:else}
							<TrainingMetricsChartLine
								height={300}
								width={chartWidth}
								values={groupMetricValues(metric)}
								unit={metric.unit}
								format={metricValuesDisplayFormat(metric)}
								average={'average' in metric.summary ? some(metric.summary.average) : none()}
								timeDomain={some(dates)}
							/>
						{/if}
					{:else}
						<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
					{/if}
				</div>
			{/each}
		{/await}
	{/if}
</div>
