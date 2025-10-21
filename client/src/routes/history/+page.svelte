<script lang="ts">
	import ActivitiesList from '../../organisms/ActivitiesList.svelte';
	import ActivitiesCalendar from '../../organisms/ActivitiesCalendar.svelte';
	import type { PageProps } from './$types';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { dayjs } from '$lib/duration';

	let { data }: PageProps = $props();

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	// View mode from URL parameter, default to 'list'
	let viewMode = $derived(
		(page.url.searchParams.get('view') === 'calendar' ? 'calendar' : 'list') as 'list' | 'calendar'
	);

	// Current month from URL parameter, default to current month
	let currentMonth = $derived.by(() => {
		const monthParam = page.url.searchParams.get('month');
		if (monthParam) {
			const parsed = dayjs(monthParam, 'YYYY-MM');
			if (parsed.isValid()) {
				return parsed.startOf('month');
			}
		}
		return dayjs().startOf('month');
	});

	const setViewMode = (mode: 'list' | 'calendar') => {
		const url = new URL(page.url);
		if (mode === 'list') {
			url.searchParams.delete('view');
		} else {
			url.searchParams.set('view', mode);
		}
		goto(url, { replaceState: true, keepFocus: true });
	};

	const handleMonthChange = (month: ReturnType<typeof dayjs>) => {
		const url = new URL(page.url);
		// If going to current month, remove the parameter
		if (month.isSame(dayjs().startOf('month'), 'month')) {
			url.searchParams.delete('month');
		} else {
			url.searchParams.set('month', month.format('YYYY-MM'));
		}
		goto(url, { replaceState: true, keepFocus: true });
	};
</script>

<div class="mx-2 mt-5 sm:mx-auto">
	<!-- View Toggle -->
	<div class="mb-4 flex items-center justify-between">
		<h1 class="text-2xl font-bold">Past activities</h1>
		<div class="join">
			<button
				class="btn join-item btn-sm {viewMode === 'list' ? 'btn-active' : 'btn-ghost'}"
				onclick={() => setViewMode('list')}
			>
				<span class="text-lg">☰</span>
				<span class="ml-1">List</span>
			</button>
			<button
				class="btn join-item btn-sm {viewMode === 'calendar' ? 'btn-active' : 'btn-ghost'}"
				onclick={() => setViewMode('calendar')}
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
		<ActivitiesCalendar
			activityList={sorted_activities}
			{currentMonth}
			onMonthChange={handleMonthChange}
		/>
	{/if}
</div>
