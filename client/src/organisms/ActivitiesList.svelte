<script lang="ts">
	import type { ActivityList } from '../routes/+page';
	import ActivitiesListItem from './ActivitiesListItem.svelte';

	let { activityList }: { activityList: ActivityList } = $props();
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="pb-2 text-lg font-semibold tracking-wide">Past activities</div>
	<div class="flex flex-col gap-2">
		{#each activityList as activity}
			<ActivitiesListItem {activity} />
		{:else}
			<li class="p-4 pb-2 text-sm italic text-center tracking-wide opacity-60">No activities</li>
		{/each}
	</div>
</div>
