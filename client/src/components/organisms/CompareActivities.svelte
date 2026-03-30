<script lang="ts">
	import MetricsMultiSelect from '$components/molecules/MetricsMultiSelect.svelte';
	import ActivityCompareChart from '$components/organisms/ActivityCompareChart.svelte';
	import { type ActivityWithTimeseries } from '$lib/api';
	import type { Metric } from '$lib/colors';

	let { activities }: { activities: ActivityWithTimeseries[] } = $props();

	let offsets: Record<string, number> = $state({});

	// Drop stale offset entries when the activities list changes
	$effect(() => {
		const ids = new Set(activities.map((a) => a.id));
		for (const id of Object.keys(offsets)) {
			if (!ids.has(id)) delete offsets[id];
		}
	});

	let chartWidth = $state(0);
	let chartHeight = $derived.by(() => {
		if (chartWidth < 640) return 240;
		if (chartWidth < 1024) return 320;
		return 400;
	});

	let metricOptions: { option: Metric; display: string }[] = [
		{ option: 'HeartRate', display: 'Heart rate' },
		{ option: 'Speed', display: 'Speed' },
		{ option: 'Power', display: 'Power' },
		{ option: 'Altitude', display: 'Altitude' },
		{ option: 'Cadence', display: 'Cadence' }
	];

	let possibleMetricOptions = $derived.by(() => {
		let metrics = [];
		const activityHasMetric = (activity: ActivityWithTimeseries, targetMetric: string) => {
			for (const metric in activity.timeseries.metrics) {
				if (metric === targetMetric) {
					return activity.timeseries.metrics[metric].values.some((value) => value !== null);
				}
			}
			return false;
		};
		for (const metric of metricOptions) {
			if (activities.some((activity) => activityHasMetric(activity, metric.option))) {
				metrics.push(metric);
			}
		}
		return metrics;
	});
	// svelte-ignore state_referenced_locally
	let selectedMetricOptions = $state(possibleMetricOptions);
</script>

{#if activities.length > 0}
	<h2 class="pb-1 text-lg">Metrics</h2>
	<MetricsMultiSelect
		availableOptions={possibleMetricOptions}
		bind:selectedOptions={selectedMetricOptions}
		useMetricColors={false}
	/>
	<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
		{#each selectedMetricOptions as metric, idx}
			<h2 class="mt-2 mb-1 text-center text-lg">{metric.display}</h2>
			<ActivityCompareChart
				{activities}
				metric={metric.option}
				width={chartWidth}
				height={chartHeight}
				{offsets}
				showOffsetControls={idx === 0}
			/>
		{/each}
	</div>
{/if}
