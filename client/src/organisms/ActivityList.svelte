<script lang="ts">
	import ActivityOverview from '../molecules/ActivityOverview.svelte';
	import type { ActivityList } from '../routes/+page';

	let { activityList }: { activityList: ActivityList } = $props();
</script>

<ul class="list rounded-box bg-base-100 shadow-md">
	<li class="p-4 pb-2 text-xs tracking-wide opacity-60">Past activities</li>
	{#each activityList as activity}
		<a href={`/activity/${activity.id}`} class="block">
			<li class="list-row bg-base-100 hover:bg-base-200">
				<ActivityOverview {activity} />
			</li>
		</a>
	{:else}
		<li class="p-4 pb-2 text-sm italic text-center tracking-wide opacity-60">No activities</li>
	{/each}
</ul>
