<script lang="ts">
	import { formatDuration, formatDateTime } from '$lib/duration';
	import TimeseriesChart from '../../../organisms/TimeseriesChart.svelte';
	import type { PageProps } from './$types';
	import Chip from '../../../molecules/Chip.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditableString from '../../../molecules/EditableString.svelte';
	import MultiSelect from '../../../molecules/MultiSelect.svelte';
	import type { Metric } from '$lib/colors';

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

	let metricOptions: { option: Metric; display: string }[] = [
		{ option: 'HeartRate', display: 'Heart rate' },
		{ option: 'Speed', display: 'Speed' },
		{ option: 'Power', display: 'Power' },
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

	let selectedOptions = $state([availableOptions[0]]);

	let selectedMetrics = $derived.by(() => {
		return selectedOptions.map((option) => {
			return {
				values: data.activity.timeseries.metrics[option.option!].values,
				name: option.option,
				unit: data.activity.timeseries.metrics[option.option!].unit
			};
		});
	});

	const deleteActivityCallback = async (): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}`, {
			method: 'DELETE',
			mode: 'cors',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
		}

		goto('/');
	};

	const updateActivityNameCallback = async (newName: string) => {
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?name=${encodeURIComponent(newName)}`,
			{
				method: 'PATCH'
			}
		);

		if (res.status === 401) {
			goto('/login');
		}
	};
</script>

<div class="mx-auto mt-1 flex max-w-2xl flex-col gap-4 sm:mt-8 sm:px-4">
	<div class="rounded-box bg-base-100 p-4 pt-3 shadow-md">
		<h1 class="text-xl">
			<EditableString content={summary?.title} editCallback={updateActivityNameCallback} />
		</h1>

		<div class="chip-container flex flex-row gap-1 overflow-auto pt-1 pl-2">
			<Chip text={summary?.sport} />
			<Chip text={`⌛ ${summary?.duration}`} />
			<Chip text={`📅 ${formatDateTime(summary?.start_time ?? '')}`} />
			<button onclick={deleteActivityCallback}>🗑️</button>
		</div>
	</div>

	<div class="rounded-box bg-base-100 p-4 pt-0 shadow-md">
		{#if data.activity}
			<fieldset class="fieldset">
				<legend class="fieldset-legend text-lg">Metrics</legend>
				<MultiSelect {availableOptions} maxSelected={3} bind:selectedOptions />
			</fieldset>
			{#if selectedMetrics}
				<div bind:clientWidth={chartWidth}>
					<TimeseriesChart
						time={data.activity.timeseries.time}
						metrics={selectedMetrics}
						height={400}
						width={chartWidth}
					/>
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.chip-container > :global(div) {
		flex-shrink: 0;
	}
</style>
