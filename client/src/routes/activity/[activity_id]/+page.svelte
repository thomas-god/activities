<script lang="ts">
	import { formatDuration, formatDateTime } from '$lib/duration';
	import TimeseriesChart from '../../../organisms/TimeseriesChart.svelte';
	import type { PageProps } from './$types';
	import Chip from '../../../molecules/Chip.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditableString from '../../../molecules/EditableString.svelte';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let summary = $derived.by(() => {
		return {
			sport: data.activity.sport,
			duration: formatDuration(data.activity.duration),
			start_time: data.activity.start_time,
			title:
				data.activity.name === null || data.activity.name === ''
					? data.activity.sport
					: data.activity.name
		};
	});

	let metricOptions = [
		{ option: 'Power', display: 'Power' },
		{ option: 'Speed', display: 'Speed' },
		{ option: 'HeartRate', display: 'Heart rate' },
		{ option: 'Altitude', display: 'Altitude' }
	];

	let availableOptions = $derived.by(() => {
		let options = [];
		for (const option of metricOptions) {
			if (
				option.option in data.activity.timeseries.metrics &&
				data.activity.timeseries.metrics[option.option].values.some((value) => value !== null)
			) {
				options.push(option);
			}
		}
		return options;
	});

	let selectedOption = $state(availableOptions.at(0));

	let selectedMetric = $derived.by(() => {
		return data.activity.timeseries.metrics[selectedOption?.option!];
	});

	const deleteActivityCallback = async (): Promise<void> => {
		await fetch(`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}`, {
			method: 'DELETE'
		});
		goto('/');
	};

	const updateActivityNameCallback = async (newName: string) => {
		await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?name=${encodeURIComponent(newName)}`,
			{
				method: 'PATCH'
			}
		);
	};
</script>

<h1 class="m-3 text-xl">
	<EditableString content={summary?.title} editCallback={updateActivityNameCallback} />
</h1>

<div class="m-3">
	<Chip text={summary?.sport} />
	<Chip text={`⌛ ${summary?.duration}`} />
	<Chip text={`📅 ${formatDateTime(summary?.start_time ?? '')}`} />
	<button onclick={deleteActivityCallback}>🗑️</button>
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
