<script lang="ts">
	import type { ActivityList, ActivityListItem } from '../routes/+page';
	import { dayjs } from '$lib/duration';
	import ActivitiesListItem from './ActivitiesListItem.svelte';

	let { activityList }: { activityList: ActivityList } = $props();

	let historyStartMonth = $derived(dayjs(activityList.at(-1)?.start_time).startOf('month'));
	let historyEndMonth = dayjs().startOf('month');

	let activitiesByMonth = $derived.by(() => {
		const activities: Map<string, ActivityListItem[]> = new Map();

		let date = historyEndMonth;
		while (date >= historyStartMonth) {
			activities.set(date.format('MMMM YYYY'), []);
			date = date.subtract(1, 'month');
		}

		for (const activity of activityList) {
			let activityStart = dayjs(activity.start_time).format('MMMM YYYY');
			activities.get(activityStart)?.push(activity);
		}

		return activities;
	});
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="pb-2 text-lg font-semibold tracking-wide">Past activities</div>

	<div class="flex flex-col gap-2">
		{#each activitiesByMonth as [month, activities]}
			<div class="flex flex-col gap-2">
				<div class="my-3 text-xs font-semibold tracking-wide text-base-content/60 uppercase">
					{month} - {activities.length} activities
				</div>
				{#each activities as activity}
					<ActivitiesListItem {activity} />
				{/each}
			</div>
		{/each}
	</div>
</div>
