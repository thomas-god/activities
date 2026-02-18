<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<div class="mx-auto pt-5 sm:px-4">
	{#await data.activity}
		<div class="flex w-full flex-col items-center p-4 pt-6">
			<div class="loading loading-bars"></div>
		</div>
	{:then activity}
		{#if activity}
			<ActivityDetails
				{activity}
				onActivityUpdated={() => {
					invalidate(`app:activity:${activity.id}`);
				}}
				onActivityDeleted={() => goto('/')}
			/>
		{:else}
			<div class="bg-warning p-4 text-warning-content">
				An error occured when trying to load this activity
				<button class="btn mt-4 btn-accent">
					<a href="/"> Go home </a>
				</button>
			</div>
		{/if}
	{/await}
</div>
