<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { type Activity, type ActivityList } from '$lib/api';
	import { dayjs, formatRelativeDuration } from '$lib/duration';
	import { onMount } from 'svelte';
	import ActivitiesFilters from './ActivitiesFilters.svelte';
	import { emptyFilters } from '$lib/filters';
	import { getSportCategoryIcon } from '$lib/sport';

	let {
		activities,
		selectedActivities = $bindable()
	}: { activities: ActivityList; selectedActivities: ActivityList } = $props();

	let filteredActivities: ActivityList = $state([]);
	let filters = $state(emptyFilters());
	let searchText: null | string = $state(null);

	let activityMatchesSearch = (activity: Activity): boolean => {
		if (searchText === null || searchText.trim() === '') {
			return true;
		}
		if (activity.name !== null) {
			return activity.name.toLowerCase().includes(searchText.trim().toLowerCase());
		}
		return false;
	};

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

<h2 class="mb-1 text-lg">Selected activities</h2>
<div class="overflow-scroll">
	{#each selectedActivities as activity}
		<div class="flex flex-row items-center gap-2 py-0.5">
			<div class="shrink-0">
				<img
					src={`/icons/${getSportCategoryIcon(activity.sport_category)}`}
					class="h-6 w-6"
					alt="Sport icon"
				/>
			</div>
			<button
				class="btn mr-1 shrink-0 btn-xs btn-secondary"
				onclick={() => {
					removeActivity(activity);
				}}>-</button
			>
			<div class="shrink-0">
				{activity.name || activity.sport}
			</div>
			<div class="shrink-0 text-sm tracking-wide italic opacity-70">
				{formatRelativeDuration(dayjs(activity.start_time), dayjs())}
			</div>
		</div>
	{:else}
		<p class="italic">No activity selected</p>
	{/each}
</div>

<div class="mt-3 mb-1 flex flex-row flex-wrap items-center gap-2">
	<h2 class="text-lg">Available activities</h2>
	<div class="flex flex-row gap-1">
		<label class="input input-sm">
			<svg class="h-[1em] opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
				<g
					stroke-linejoin="round"
					stroke-linecap="round"
					stroke-width="2.5"
					fill="none"
					stroke="currentColor"
				>
					<circle cx="11" cy="11" r="8"></circle>
					<path d="m21 21-4.3-4.3"></path>
				</g>
			</svg>
			<input type="search" class="grow" placeholder="Search" bind:value={searchText} />
		</label>
		<ActivitiesFilters {activities} bind:filters bind:filteredActivities showLabel={false} />
	</div>
</div>
<div class="max-h-64 overflow-scroll">
	{#each filteredActivities as activity (activity.id)}
		{#if !selectedIds.includes(activity.id) && activityMatchesSearch(activity)}
			<div class="flex flex-row items-center gap-2 py-0.5">
				<div class="shrink-0">
					<img
						src={`/icons/${getSportCategoryIcon(activity.sport_category)}`}
						class="h-6 w-6"
						alt="Sport icon"
					/>
				</div>
				<button
					class="btn mr-1 shrink-0 btn-xs btn-primary"
					onclick={() => {
						selectActivity(activity);
					}}>+</button
				>
				<div class="shrink-0">
					{activity.name || activity.sport}
				</div>
				<div class="shrink-0 text-sm tracking-wide italic opacity-70">
					{formatRelativeDuration(dayjs(activity.start_time), dayjs())}
				</div>
			</div>
		{/if}
	{/each}
</div>
