<script lang="ts">
	import ActivitiesCalendar from '$components/organisms/ActivitiesCalendar.svelte';
	import DownloadActivitiesModal from '$components/molecules/DownloadActivitiesModal.svelte';
	import type { PageProps } from './$types';
	import { page } from '$app/state';
	import { goto, invalidate } from '$app/navigation';
	import { dayjs } from '$lib/duration';
	import {
		fetchActivityDetails,
		fetchActivityListSummary,
		setPreference,
		type ActivityListSummaryItems,
		type PreferencePayload
	} from '$lib/api';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import type { ActivityList, ActivityWithTimeseries } from '$lib/api/activities';
	import Timeline from '$components/pages/Timeline.svelte';
	import ActivitiesFiltersComponent from '$components/molecules/ActivitiesFilters.svelte';
	import { filtersFromSearchParams, applyFiltersToSearchParams } from '$lib/filters';
	import type { ActivitiesFilters } from '$lib/filters';
	import NavbarActivities from '$components/organisms/navigation/NavbarActivities.svelte';
	import ActivityListSummaryDialog from '$components/organisms/ActivityListSummaryDialog.svelte';

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
			selectedActivityId = null;
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

	const handleActivityDeleted = (activityId: string) => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		activities = activities.filter((activity) => activity.id !== activityId);
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

	const updateActivityListSummary = async (items: ActivityListSummaryItems): Promise<boolean> => {
		const payload: PreferencePayload = {
			key: 'activity_list_summary',
			value: {
				scope: { type: 'global' },
				items
			}
		};

		return setPreference(fetch, payload).then((res) => {
			invalidate('app:activities');
			return res;
		});
	};

	let summaryDialog: ActivityListSummaryDialog | null = $state(null);
	const openSummaryDialog = () => {
		if (summaryDialog !== null) {
			summaryDialog.open();
		}
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="flex flex-col">
	<NavbarActivities
		invalidateActivities={() => invalidate('app:activities')}
		invalidateTrainingNotes={() => invalidate('app:training-notes')}
	/>

	<div class="flex flex-row items-start gap-2">
		<div class="@container/main mt-5 flex grow flex-col rounded-box bg-base-100 px-4 shadow-md">
			<!-- View Toggle -->
			<div
				class="sticky top-0 flex flex-col justify-between gap-2 bg-base-100 py-4 @sm/main:flex-row @sm:items-center"
			>
				<h1 class="hidden text-2xl font-bold @sm/main:block">History</h1>
				<div class="flex gap-0.5 sm:gap-2">
					<div class="join">
						<button
							class="btn join-item btn-sm {viewMode === 'list' ? 'btn-active' : 'btn-ghost'}"
							onclick={() => setViewMode('list')}
						>
							<img src="/icons/list.svg" class="h-5 w-5" alt="List icon" />
							<span class="ml-1 hidden @sm/main:inline">List</span>
						</button>
						<button
							class="btn join-item btn-sm {viewMode === 'calendar' ? 'btn-active' : 'btn-ghost'}"
							onclick={() => setViewMode('calendar')}
						>
							<img src="/icons/calendar.svg" class="h-5 w-5" alt="Calendar icon" />
							<span class="ml-1 hidden @sm/main:inline">Calendar</span>
						</button>
					</div>
					<div class="join">
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
							class="btn join-item btn-sm"
							onclick={openSummaryDialog}
							title="Customize history view"
						>
							<img src="/icons/power.svg" class="h-6 w-6" alt="Gear/customize icon" />
							<span class="ml-1 hidden @min-[600px]:inline">Customize</span>
						</button>
						<button
							class="btn join-item btn-sm"
							onclick={handleDownloadClick}
							title="Download all activities as ZIP"
						>
							<img src="/icons/download.svg" class="h-6 w-6" alt="Download icon" />
							<span class="ml-1 hidden @min-[600px]:inline">Download</span>
						</button>
					</div>
				</div>
			</div>

			<!-- View Content -->
			{#await Promise.all([data.activities, data.notes, fetchActivityListSummary(fetch)])}
				<div class="flex w-full flex-col items-center p-4 pt-6">
					<div class="loading loading-bars"></div>
				</div>
			{:then [_, notes, activityListFormat]}
				{#if viewMode === 'list'}
					<Timeline
						activities={filteredActivities}
						{notes}
						{selectedActivityId}
						selectActivityCallback={handleActivitySelected}
						{activityListFormat}
						noteChangedCallback={() => invalidate('app:training-notes')}
						renderByChunk={true}
					/>
				{:else}
					<ActivitiesCalendar
						activityList={filteredActivities}
						onActivitySelected={handleActivitySelected}
						{currentMonth}
						onMonthChange={handleMonthChange}
					/>
				{/if}

				<DownloadActivitiesModal
					bind:isOpen={showDownloadModal}
					activityCount={activities.length}
				/>
			{/await}
		</div>

		{#if selectedActivityPromise && screenWidth >= 700}
			<div
				class="selected-activity relative mt-5 w-full grow basis-0 overflow-auto rounded-box bg-base-100 p-4 pt-4 shadow-md"
			>
				{#await selectedActivityPromise}
					<div class="flex items-center justify-center">
						<span class="loading loading-lg loading-spinner"></span>
					</div>
				{:then selectedActivity}
					{#if selectedActivity}
						<div class="absolute right-3 join">
							<button
								onclick={() => goto(`/activity/${selectedActivityId}`)}
								class="btn join-item btn-sm"
							>
								<img src="/icons/expand.svg" alt="Close icon" class="inline h-4 w-4" /></button
							>
							<button
								onclick={() => {
									handleActivitySelected(null);
								}}
								class="btn join-item btn-sm"
							>
								<img src="/icons/close.svg" alt="Close icon" class="inline h-4 w-4" /></button
							>
						</div>
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
</div>

{#await Promise.all( [data.defaultMetrics, data.activityListSummary] ) then [defaultMetrics, currentPreference]}
	<ActivityListSummaryDialog
		bind:this={summaryDialog}
		{defaultMetrics}
		{currentPreference}
		onSave={updateActivityListSummary}
	/>
{/await}

<style>
	.selected-activity {
		height: calc(80vh + 64px);
	}
</style>
