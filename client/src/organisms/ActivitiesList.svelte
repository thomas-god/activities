<script lang="ts">
	import type { ActivityList, ActivityListItem } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import ActivitiesListItem from './ActivitiesListItem.svelte';
	import ActivitiesFilters from '../molecules/ActivitiesFilters.svelte';

	let { activityList }: { activityList: ActivityList } = $props();

	let filteredActivityList = $state<ActivityList>(activityList);

	let historyStartMonth = $derived(dayjs(filteredActivityList.at(-1)?.start_time).startOf('month'));
	let historyEndMonth = dayjs().startOf('month');

	let activitiesByMonth = $derived.by(() => {
		const activities: Map<string, ActivityListItem[]> = new Map();

		let date = historyEndMonth;
		while (date >= historyStartMonth) {
			activities.set(date.format('MMMM YYYY'), []);
			date = date.subtract(1, 'month');
		}

		for (const activity of filteredActivityList) {
			let activityStart = dayjs(activity.start_time).format('MMMM YYYY');
			activities.get(activityStart)?.push(activity);
		}

		return activities;
	});

	const handleFilterChange = (filtered: ActivityList) => {
		filteredActivityList = filtered;
	};
</script>

<div class="rounded-box bg-base-100 shadow-md">
	<ActivitiesFilters activities={activityList} onFilterChange={handleFilterChange} />
	<div class="flex flex-col gap-2 p-4 pt-0">
		{#if filteredActivityList.length === 0}
			<div class="py-8 text-center text-base-content/60">
				No activities match the selected filters
			</div>
		{:else}
			{#each activitiesByMonth as [month, activities]}
				{#if activities.length > 0}
					<div class="flex flex-col gap-2">
						<div class="my-3 text-xs font-semibold tracking-wide text-base-content/60 uppercase">
							{month} - {activities.length} activities
						</div>
						{#each activities as activity}
							<ActivitiesListItem {activity} />
						{/each}
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>
