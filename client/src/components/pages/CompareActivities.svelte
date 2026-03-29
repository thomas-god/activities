<script lang="ts">
	import MetricsMultiSelect from '$components/molecules/MetricsMultiSelect.svelte';
	import ActivityCompareChart from '$components/organisms/ActivityCompareChart.svelte';
	import { type ActivityWithTimeseries } from '$lib/api';
	import type { Metric } from '$lib/colors';

	let { activities }: { activities: ActivityWithTimeseries[] } = $props();

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
	<MetricsMultiSelect
		availableOptions={possibleMetricOptions}
		bind:selectedOptions={selectedMetricOptions}
		useMetricColors={false}
	/>
	<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
		{#each selectedMetricOptions as metric}
			<h2 class="text-center text-lg">{metric.display}</h2>
			<ActivityCompareChart
				{activities}
				metric={metric.option}
				width={chartWidth}
				height={chartHeight}
			/>
		{/each}
	</div>
{/if}
