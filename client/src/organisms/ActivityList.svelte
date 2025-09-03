<script lang="ts">
	import type { ActivityList } from '$lib/types/activity';
	import ActivityOverview from '../molecules/ActivityOverview.svelte';

	let { activityList }: { activityList: ActivityList } = $props();
</script>

{#each activityList as activity}
	<ActivityOverview {activity} />
{/each}
