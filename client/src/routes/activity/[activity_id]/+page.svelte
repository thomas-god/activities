<script lang="ts">
	import { formatDuration, formatDateTime } from '$lib/duration';
	import TimeseriesChart from '../../../molecules/TimeseriesChart.svelte';
	import type { PageProps } from './$types';
  	import Chip from '../../../molecules/Chip.svelte';

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

	let availableOptions = $derived.by(() => {
		if (data.activity === undefined) {
			return [];
		}

		let options = [];
		for (const option of metricOptions) {
			if (option.option in data.activity.timeseries.metrics) {
				options.push(option);
			}
		}
		return options;
	});

	let selectedOption = $state(metricOptions.at(1));

	let selectedMetric = $derived.by(() => {
		if (data.activity === undefined) {
			return undefined;
		}

		return data.activity.timeseries.metrics[selectedOption?.option!];
	});
</script>

<div class="m-3">
  <Chip text={summary?.sport} />
  <Chip text={`⌛ ${summary?.duration}`} />
  <Chip text={`📅 ${formatDateTime(summary?.start_time ?? '')}`} />
</div>

<div class="m-3">
	{#if data.activity}
		<fieldset class="fieldset">
			<legend class="fieldset-legend">Metrics</legend>
			<select class="select" bind:value={selectedOption}>
				{#each availableOptions as option (option.option)}
					<option value={option}>{option.display}</option>
				{/each}
			</select>
		</fieldset>
		{#if selectedMetric}
			<div bind:clientWidth={chartWidth}>
				<TimeseriesChart
					time={data.activity.timeseries.time}
					metric={selectedMetric.values}
					unit={selectedMetric.unit}
					height={400}
					width={chartWidth}
				/>
			</div>
		{/if}
	{/if}
</div>
