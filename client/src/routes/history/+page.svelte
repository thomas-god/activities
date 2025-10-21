<script lang="ts">
	import ActivitiesList from '../../organisms/ActivitiesList.svelte';
	import ActivitiesCalendar from '../../organisms/ActivitiesCalendar.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	// View mode state: 'list' or 'calendar'
	let viewMode = $state<'list' | 'calendar'>('list');
</script>

<div class="mx-2 mt-5 sm:mx-auto">
	<!-- View Toggle -->
	<div class="mb-4 flex items-center justify-between">
		<h1 class="text-2xl font-bold">Activity History</h1>
		<div class="join">
			<button
				class="btn join-item btn-sm {viewMode === 'list' ? 'btn-active' : 'btn-ghost'}"
				onclick={() => (viewMode = 'list')}
			>
				<span class="text-lg">☰</span>
				<span class="ml-1">List</span>
			</button>
			<button
				class="btn join-item btn-sm {viewMode === 'calendar' ? 'btn-active' : 'btn-ghost'}"
				onclick={() => (viewMode = 'calendar')}
			>
				<span class="text-lg">📅</span>
				<span class="ml-1">Calendar</span>
			</button>
		</div>
	</div>

	<!-- View Content -->
	{#if viewMode === 'list'}
		<ActivitiesList activityList={sorted_activities} />
	{:else}
		<ActivitiesCalendar activityList={sorted_activities} />
	{/if}
</div>
