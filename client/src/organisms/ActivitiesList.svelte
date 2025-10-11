<script lang="ts">
	import { formatDuration, localiseDate } from '$lib/duration';
	import type { ActivityList, ActivityListItem } from '../routes/+page';

	let { activityList }: { activityList: ActivityList } = $props();

	const activityTitle = (activity: ActivityListItem) => {
		if (activity.name === null || activity.name === '') {
			return activity.sport;
		}
		return activity.name;
	};
</script>

<ul class="rounded-box bg-base-100 shadow-md">
	<li class="p-4 pb-2 text-xs tracking-wide opacity-60">Past activities</li>
	{#each activityList as activity}
		<a href={`/activity/${activity.id}`} class="block">
			<li class="grid grid-cols-3 justify-self-auto bg-base-100 p-3 hover:bg-base-200">
				<div class="italic">
					{activityTitle(activity)}
				</div>
				<div class="justify-self-start">
					⌛ <span class="font-mono font-medium italic">{formatDuration(activity.duration)}</span>
				</div>
				<div class="justify-self-end">
					📅 <span class="italic">{localiseDate(activity.start_time)}</span>
				</div>
			</li>
		</a>
	{:else}
		<li class="p-4 pb-2 text-sm italic text-center tracking-wide opacity-60">No activities</li>
	{/each}
</ul>
