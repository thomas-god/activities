<script lang="ts">
	import { formatDuration, formatRelativeDuration } from '$lib/duration';
	import dayjs from 'dayjs';
	import type { ActivityList } from '../routes/+page';

	let { activityList }: { activityList: ActivityList } = $props();
</script>

<ul class="rounded-box bg-base-100 shadow-md">
	<li class="p-4 pb-2 text-xs tracking-wide opacity-60">Past activities</li>
	{#each activityList as activity}
		<a href={`/activity/${activity.id}`} class="block">
			<li class="bg-base-100 hover:bg-base-200 flex flex-row gap-3 p-3">
				<div class="basis-20 text-end">
					{activity.sport}:
				</div>

				<div class="grow">
					{`${formatDuration(activity.duration)}`}
				</div>

				<div class="italic">
					{formatRelativeDuration(dayjs(activity.start_time), dayjs())}
				</div>
			</li>
		</a>
	{:else}
		<li class="p-4 pb-2 text-sm italic text-center tracking-wide opacity-60">No activities</li>
	{/each}
</ul>
