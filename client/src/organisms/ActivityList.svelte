<script lang="ts">
	import type { ActivityList } from '$lib/types/activity';
	import ActivityOverview from '../molecules/ActivityOverview.svelte';

	let { activityList }: { activityList: ActivityList } = $props();
</script>

<ul class="list">
	<li>Last activities</li>
	{#each activityList as activity}
		<li class="list-row">
			<ActivityOverview {activity} />
		</li>
	{/each}
</ul>
