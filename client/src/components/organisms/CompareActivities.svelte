<script lang="ts">
	import * as d3 from 'd3';
	import MetricsMultiSelect from '$components/molecules/MetricsMultiSelect.svelte';
	import OffsetControls from '$components/molecules/OffsetControls.svelte';
	import ActivityCompareChart from '$components/organisms/ActivityCompareChart.svelte';
	import { type ActivityWithTimeseries } from '$lib/api';
	import type { Metric } from '$lib/colors';
	import { SvelteMap } from 'svelte/reactivity';

	let { activities }: { activities: ActivityWithTimeseries[] } = $props();

	let offsets: SvelteMap<string, number> = $state(new SvelteMap());
	let offsetsDialogElement: HTMLDialogElement;
	let offsetsDialogElementWidth = $state(0);
	let offsetsDialogMetric = $state('Altitude');

	// Drop stale offset entries when the activities list changes
	$effect(() => {
		const ids = new Set(activities.map((a) => a.id));
		for (const id of Object.keys(offsets)) {
			if (!ids.has(id)) {
				offsets.delete(id);
			}
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
	<h2 class="pb-1 text-lg">
		Metrics
		<button class="btn btn-ghost btn-sm" onclick={() => offsetsDialogElement.show()}>
			<img src="/icons/power.svg" alt="Cog icon" class="h-6 w-6" />
		</button>
	</h2>
	<MetricsMultiSelect
		availableOptions={possibleMetricOptions}
		bind:selectedOptions={selectedMetricOptions}
		useMetricColors={false}
	/>
	<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
		{#each selectedMetricOptions as metric}
			<h2 class="mt-2 mb-1 text-center text-lg">{metric.display}</h2>
			<ActivityCompareChart
				{activities}
				metric={metric.option}
				width={chartWidth}
				height={chartHeight}
				{offsets}
			/>
		{/each}
	</div>
{/if}

<dialog id="my_modal_2" class="modal" bind:this={offsetsDialogElement}>
	<div class="modal-box flex flex-col gap-3 p-3 sm:p-6">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<h3 class="text-lg font-bold">Timeseries alignment</h3>
		<p class="opacity-75">You can align timeseries by adding a time offset for each activity.</p>
		<select class="select" bind:value={offsetsDialogMetric}>
			{#each possibleMetricOptions as metric}
				<option value={metric.option}>{metric.display}</option>
			{/each}
		</select>
		<OffsetControls
			bind:offsets
			activities={activities.map((activity, idx) => ({
				label: activity.name ?? activity.start_time.slice(0, 10),
				id: activity.id,
				color: d3.schemeTableau10[idx % d3.schemeTableau10.length]
			}))}
		/>
		<div bind:clientWidth={offsetsDialogElementWidth}>
			<ActivityCompareChart
				{activities}
				metric={offsetsDialogMetric}
				width={offsetsDialogElementWidth}
				height={chartHeight}
				{offsets}
			/>
		</div>
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
