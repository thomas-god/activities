<script lang="ts">
	import { formatDuration, formatRelativeDuration } from '$lib/duration';
	import dayjs from 'dayjs';
	import type { ActivityList, ActivityListItem } from '../routes/+page';

	let { activityList, moreCallback }: { activityList: ActivityList; moreCallback: () => void } =
		$props();

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
					📅 <span class="italic"
						>{formatRelativeDuration(dayjs(activity.start_time), dayjs())}</span
					>
				</div>
			</li>
		</a>
	{:else}
		<li class="p-4 pb-2 text-sm italic text-center tracking-wide opacity-60">No activities</li>
	{/each}
	<li class="px-4 pt-0 pb-2 text-xs tracking-wide opacity-60">
		<button class="btn btn-link btn-sm" onclick={moreCallback}> + more </button>
	</li>
</ul>
