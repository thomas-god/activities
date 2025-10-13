<script lang="ts">
	import ActivitiesList from '../../organisms/ActivitiesList.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);
</script>

<div class="mx-2 mt-5 sm:mx-auto">
	<ActivitiesList activityList={sorted_activities} />
</div>
