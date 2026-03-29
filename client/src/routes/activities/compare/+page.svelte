<script lang="ts">
	import ActivitiesSelect from '$components/molecules/ActivitiesSelect.svelte';
	import ActivityCompareChart from '$components/organisms/ActivityCompareChart.svelte';
	import { fetchActivityDetails, type ActivityList, type ActivityWithTimeseries } from '$lib/api';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let selectedActivities: ActivityList = $state([]);
	let activitiesDetailsPromise: Promise<Array<ActivityWithTimeseries | null>> = $state(
		Promise.resolve([])
	);
	let activitiesDetails = $derived(await activitiesDetailsPromise);

	let loadedActivities = $derived(
		activitiesDetails.filter((a): a is ActivityWithTimeseries => a !== null)
	);

	let chartWidth = $state(0);
	let chartHeight = $derived.by(() => {
		if (chartWidth < 640) return 240;
		if (chartWidth < 1024) return 320;
		return 400;
	});
</script>

{#await data.activities}
	<div class="loading-dots"></div>
{:then activities}
	<ActivitiesSelect
		{activities}
		bind:selectedActivities={
			() => selectedActivities,
			(activities) => {
				selectedActivities = activities;
				activitiesDetailsPromise = Promise.all(
					activities.map((activity) => fetchActivityDetails(fetch, activity.id))
				);
			}
		}
	/>
{/await}

{#if loadedActivities.length > 0}
	<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
		{#each ['HeartRate', 'Power', 'Altitude', 'Cadence', 'Pace', 'Speed'] as metric}
			<h2 class="text-center text-lg">{metric}</h2>
			<ActivityCompareChart
				activities={loadedActivities}
				{metric}
				width={chartWidth}
				height={chartHeight}
			/>
		{/each}
	</div>
{/if}
