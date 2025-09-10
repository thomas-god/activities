<script lang="ts">
	import { formatDuration } from '$lib/duration';
	import TimeseriesChart from '../../../molecules/TimeseriesChart.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let summary = $derived.by(() => {
		if (data.activity) {
			return {
				sport: data.activity.sport,
				duration: formatDuration(data.activity.duration),
				start_time: data.activity.start_time
			};
		}
		return undefined;
	});

	let metricOptions = [
		{ option: 'Power', display: 'Power' },
		{ option: 'Speed', display: 'Speed' },
		{ option: 'HeartRate', display: 'Heart rate' }
	];

	let selectedOption = $state(metricOptions.at(1));
</script>

{JSON.stringify(summary)}

<div class="m-3">
	{#if data.activity}
		<fieldset class="fieldset">
			<legend class="fieldset-legend">Metrics</legend>
			<select class="select" bind:value={selectedOption}>
				{#each metricOptions as option (option.option)}
					<option value={option}>{option.display}</option>
				{/each}
			</select>
		</fieldset>
		{#if selectedOption}
			<div bind:clientWidth={chartWidth}>
				<TimeseriesChart
					timeseries={data.activity.timeseries}
					targetMetric={selectedOption.option}
					height={400}
					width={chartWidth}
				/>
			</div>
		{/if}
	{/if}
</div>
