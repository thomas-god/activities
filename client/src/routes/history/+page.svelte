<script lang="ts">
	import ActivitiesCalendar from '$components/organisms/ActivitiesCalendar.svelte';
	import DownloadActivitiesModal from '$components/molecules/DownloadActivitiesModal.svelte';
	import type { PageProps } from './$types';
	import { page } from '$app/state';
	import { goto, invalidate } from '$app/navigation';
	import { dayjs } from '$lib/duration';
	import { fetchActivityDetails } from '$lib/api';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import type { ActivityList, ActivityWithTimeseries } from '$lib/api/activities';
	import Timeline from '$components/pages/Timeline.svelte';
	import ActivitiesFiltersComponent from '$components/molecules/ActivitiesFilters.svelte';
	import { filtersFromSearchParams, applyFiltersToSearchParams } from '$lib/filters';
	import type { ActivitiesFilters } from '$lib/filters';

	let { data }: PageProps = $props();

	let screenWidth = $state(0);
	let showDownloadModal = $state(false);

	let selectedActivityId: string | null = $state(null);
	let selectedActivityPromise: Promise<ActivityWithTimeseries | null> | null = $state(null);

	// View mode from URL parameter, default to 'list'
	let viewMode = $derived(
		(page.url.searchParams.get('view') === 'calendar' ? 'calendar' : 'list') as 'list' | 'calendar'
	);

	let activities: ActivityList = $state([]);
	$effect(() => {
		data.activities.then((a) => (activities = a));
	});
	let filteredActivities: ActivityList = $state([]);
	let filters = $derived(filtersFromSearchParams(page.url.searchParams));

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

	const handleFilterChange = (filters: ActivitiesFilters) => {
		const url = new URL(page.url);
		applyFiltersToSearchParams(url.searchParams, filters);
		goto(url, { replaceState: false, keepFocus: true });
	};

	const handleActivitySelected = (activityId: string | null) => {
		if (activityId === null) {
			activityId = null;
			selectedActivityPromise = null;
			return;
		}

		// On small screens, navigate to activity page
		if (screenWidth < 700) {
			goto(`/activity/${activityId}`);
			return;
		}

		// On larger screens, load and show activity details here
		selectedActivityId = activityId;
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};

	const handleActivityDeleted = (activiyId: string) => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		activities = activities.filter((activity) => activity.id !== activiyId);
	};

	const handleActivityUpdated = (updatedActivity: ActivityWithTimeseries) => {
		let idx = activities.findIndex((activity) => activity.id === updatedActivity.id);
		if (idx > -1) {
			activities[idx] = updatedActivity;
		}
	};

	const handleDownloadClick = () => {
		showDownloadModal = true;
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="@container mt-5 rounded-box bg-base-100 p-4 shadow-md sm:mx-auto">
	<!-- View Toggle -->
	<div class="mb-4 flex flex-col justify-between gap-2 @sm:flex-row @sm:items-center">
		<h1 class="hidden text-2xl font-bold sm:block">History</h1>
		<div class="flex gap-0.5 sm:gap-2">
			<div class="join">
				<button
					class="btn join-item btn-sm {viewMode === 'list' ? 'btn-active' : 'btn-ghost'}"
					onclick={() => setViewMode('list')}
				>
					<img src="/icons/list.svg" class="h-5 w-5" alt="List icon" />
					<span class="ml-1 hidden sm:inline">List</span>
				</button>
				<button
					class="btn join-item btn-sm {viewMode === 'calendar' ? 'btn-active' : 'btn-ghost'}"
					onclick={() => setViewMode('calendar')}
				>
					<img src="/icons/calendar.svg" class="h-5 w-5" alt="Calendar icon" />
					<span class="ml-1 hidden sm:inline">Calendar</span>
				</button>
			</div>
			{#await data.activities then _}
				<ActivitiesFiltersComponent
					{activities}
					bind:filteredActivities
					bind:filters={
						() => filters,
						(f) => {
							handleFilterChange(f);
						}
					}
				/>
			{/await}
			<button
				class="btn btn-ghost btn-sm"
				onclick={handleDownloadClick}
				title="Download all activities as ZIP"
			>
				<img src="/icons/download.svg" class="h-6 w-6" alt="Download icon" />
				<span class="ml-1 hidden sm:inline">Download</span>
			</button>
		</div>
	</div>

	<!-- View Content -->
	{#await Promise.all([data.activities, data.notes])}
		<div class="flex w-full flex-col items-center p-4 pt-6">
			<div class="loading loading-bars"></div>
		</div>
	{:then [_, notes]}
		{#if viewMode === 'list'}
			<div class="flex h-[100vh] flex-row gap-2 overflow-hidden">
				<div class="grow basis-0 overflow-y-auto">
					<Timeline
						activities={filteredActivities}
						{notes}
						{selectedActivityId}
						selectActivityCallback={handleActivitySelected}
					/>
				</div>
				{#if selectedActivityPromise && screenWidth >= 700}
					<div class="relative w-full grow basis-0 overflow-auto pt-4">
						{#await selectedActivityPromise}
							<div class="flex items-center justify-center">
								<span class="loading loading-lg loading-spinner"></span>
							</div>
						{:then selectedActivity}
							{#if selectedActivity}
								<button onclick={() => handleActivitySelected(null)} class="absolute right-3"
									>X</button
								>
								<ActivityDetails
									activity={selectedActivity}
									onActivityUpdated={handleActivityUpdated}
									onActivityDeleted={() => handleActivityDeleted(selectedActivity.id)}
									compact={true}
								/>
							{:else}
								<div
									class="flex items-center justify-center rounded-box bg-base-100 p-8 text-error shadow-md"
								>
									Failed to load activity
								</div>
							{/if}
						{:catch error}
							<div
								class="flex items-center justify-center rounded-box bg-base-100 p-8 text-error shadow-md"
							>
								Failed to load activity: {error.message}
							</div>
						{/await}
					</div>
				{/if}
			</div>
		{:else}
			<ActivitiesCalendar
				activityList={filteredActivities}
				{currentMonth}
				onMonthChange={handleMonthChange}
			/>
		{/if}

		<DownloadActivitiesModal bind:isOpen={showDownloadModal} activityCount={activities.length} />
	{/await}
</div>
