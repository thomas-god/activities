<script lang="ts">
	import { dayjs } from '$lib/duration';
	import type { ActivityList, ActivityListItem } from '../routes/+page';
	import ActivitiesListItem from './ActivitiesListItem.svelte';

	let { activityList, moreCallback }: { activityList: ActivityList; moreCallback: () => void } =
		$props();

	let groupedActivities = $derived.by(() => {
		const now = dayjs();
		const activities = {
			thisWeek: [] as ActivityListItem[],
			thisMonth: [] as ActivityListItem[],
			earlier: [] as ActivityListItem[]
		};

		for (const activity of activityList) {
			const start = dayjs(activity.start_time);
			if (start > now.startOf('isoWeek')) {
				activities.thisWeek.push(activity);
			} else if (start > now.startOf('month')) {
				activities.thisMonth.push(activity);
			} else {
				activities.earlier.push(activity);
			}
		}

		return activities;
	});
	const containerClass = 'text-base-content/60 my-3 text-xs font-semibold uppercase tracking-wide';
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="flex items-center justify-between pb-2 text-lg font-semibold tracking-wide">
		<span> Recent activities </span>
		<button class="btn btn-link btn-sm" onclick={moreCallback}> view all →</button>
	</div>

	{#if groupedActivities.thisWeek.length > 0}
		<p class={containerClass}>This week</p>
		<div class="flex flex-col gap-2">
			{#each groupedActivities.thisWeek as activity}
				<ActivitiesListItem {activity} />
			{/each}
		</div>
	{/if}

	{#if groupedActivities.thisMonth.length > 0}
		<p class={containerClass}>This month</p>
		<div class="flex flex-col gap-2">
			{#each groupedActivities.thisMonth as activity}
				<ActivitiesListItem {activity} />
			{/each}
		</div>
	{/if}

	{#if groupedActivities.earlier.length > 0}
		<p class={containerClass}>Earlier</p>
		<div class="flex flex-col gap-2">
			{#each groupedActivities.earlier as activity}
				<ActivitiesListItem {activity} />
			{/each}
		</div>
	{/if}
</div>
