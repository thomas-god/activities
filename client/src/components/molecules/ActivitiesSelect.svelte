<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { type Activity, type ActivityList } from '$lib/api';
	import { dayjs, formatRelativeDuration } from '$lib/duration';
	import { onMount } from 'svelte';

	let {
		activities,
		selectedActivities = $bindable()
	}: { activities: ActivityList; selectedActivities: ActivityList } = $props();

	let selectedIds: string[] = $derived(selectedActivities.map((a) => a.id));
	onMount(() => {
		if (page.url.searchParams.get('activities') !== null) {
			selectedIds = page.url.searchParams.get('activities')!.split(',');
		}
		selectedActivities = activities.filter((activity) => selectedIds.includes(activity.id));
	});

	const selectActivity = (newActivity: Activity) => {
		selectedActivities = [...selectedActivities, newActivity];
		updateUrl();
	};

	const removeActivity = (activityToRemove: Activity) => {
		selectedActivities = selectedActivities.filter(
			(activity) => activity.id !== activityToRemove.id
		);
		updateUrl();
	};

	const updateUrl = () => {
		const url = new URL(page.url);
		if (selectedActivities.length === 0) {
			url.searchParams.delete('activities');
		} else {
			url.searchParams.set(
				'activities',
				selectedActivities.map((activity) => activity.id).join(',')
			);
		}
		goto(url, { replaceState: false, keepFocus: true });
	};
</script>

<h2 class="text-lg">Selected activities</h2>
{#each selectedActivities as activity}
	<p>
		{activity.name || activity.sport}
		<span class="text-sm tracking-wide italic opacity-70"
			>{formatRelativeDuration(dayjs(activity.start_time), dayjs())}</span
		>
		<button
			onclick={() => {
				removeActivity(activity);
			}}>-</button
		>
	</p>
{:else}
	<p class="italic">No activity selected</p>
{/each}

<h2 class="mt-2 text-lg">Available activities</h2>
<div class="max-h-64 overflow-scroll">
	{#each activities as activity (activity.id)}
		{#if !selectedIds.includes(activity.id)}
			<div>
				{activity.name || activity.sport}
				<span class="text-sm tracking-wide italic opacity-70"
					>{formatRelativeDuration(dayjs(activity.start_time), dayjs())}</span
				>
				<button
					onclick={() => {
						selectActivity(activity);
					}}>+</button
				>
			</div>
		{/if}
	{/each}
</div>
