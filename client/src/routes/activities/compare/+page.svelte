<script lang="ts">
	import ActivitiesSelect from '$components/molecules/ActivitiesSelect.svelte';
	import CompareActivities from '$components/pages/CompareActivities.svelte';
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
	<CompareActivities activities={loadedActivities} />
{/if}
