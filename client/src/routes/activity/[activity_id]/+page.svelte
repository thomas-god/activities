<script lang="ts">
	import { goto } from '$app/navigation';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<div class="mx-auto pt-5 sm:px-4">
	<ActivityDetails
		activity={data.activity}
		onActivityUpdated={() => {
			// TODO: handle update path
		}}
		onActivityDeleted={() => goto('/')}
	/>
</div>
