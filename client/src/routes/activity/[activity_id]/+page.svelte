<script lang="ts">
	import { goto } from '$app/navigation';
	import Navbar from '$ui/navigation/Navbar.svelte';
	import ActivityDetails from '$ui/activity/ActivityDetails.svelte';
	import type { PageProps } from './$types';
	import { resolve } from '$app/paths';

	let { data }: PageProps = $props();
</script>

<Navbar />

<div class="mx-auto pt-5 sm:px-4">
	{#await data.activity}
		<div class="flex w-full flex-col items-center p-4 pt-6">
			<div class="loading loading-bars"></div>
		</div>
	{:then activity}
		{#if activity}
			<ActivityDetails
				{activity}
				onActivityUpdated={() => {}}
				onActivityDeleted={() => goto(resolve('/'))}
			/>
		{:else}
			<div class="bg-warning p-4 text-warning-content">
				An error occurred when trying to load this activity
				<button class="btn mt-4 btn-accent">
					<a href={resolve('/')}> Go home </a>
				</button>
			</div>
		{/if}
	{/await}
</div>
