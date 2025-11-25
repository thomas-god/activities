<script lang="ts">
	import ActivitiesList from '$components/organisms/ActivitiesList.svelte';
	import ActivitiesCalendar from '$components/organisms/ActivitiesCalendar.svelte';
	import type { PageProps } from './$types';
	import { page } from '$app/state';
	import { goto, invalidate } from '$app/navigation';
	import { dayjs } from '$lib/duration';
	import type { WorkoutType } from '$lib/workout-type';
	import type { SportCategory } from '$lib/sport';
	import { fetchActivityDetails } from '$lib/api';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import type { ActivityDetails as ActivityDetailsType } from '$lib/api/activities';

	let { data }: PageProps = $props();

	let screenWidth = $state(0);
	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);
	let selectedActivityId: string | null = $state(null);
	let selectedActivityPromise: Promise<ActivityDetailsType | null> | null = $state(null);

	// View mode from URL parameter, default to 'list'
	let viewMode = $derived(
		(page.url.searchParams.get('view') === 'calendar' ? 'calendar' : 'list') as 'list' | 'calendar'
	);

	// Parse filters from URL
	let initialRpe = $derived.by(() => {
		const rpeParam = page.url.searchParams.get('rpe');
		if (!rpeParam) return [];
		return rpeParam
			.split(',')
			.map(Number)
			.filter((n) => !isNaN(n) && n >= 1 && n <= 10);
	});

	let initialWorkoutTypes = $derived.by(() => {
		const wtParam = page.url.searchParams.get('workout_type');
		if (!wtParam) return [];
		return wtParam.split(',') as WorkoutType[];
	});

	let initialSportCategories = $derived.by(() => {
		const scParam = page.url.searchParams.get('sport_category');
		if (!scParam) return [];
		return scParam.split(',') as SportCategory[];
	});

	let initialShowNotes = $derived.by(() => {
		const param = page.url.searchParams.get('show_notes');
		return param === null ? true : param === 'true';
	});

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

	const handleFilterChange = (filters: {
		rpe: number[];
		workoutTypes: WorkoutType[];
		sportCategories: SportCategory[];
		showNotes: boolean;
	}) => {
		const url = new URL(page.url);

		// Update RPE filter
		if (filters.rpe.length > 0) {
			url.searchParams.set('rpe', filters.rpe.join(','));
		} else {
			url.searchParams.delete('rpe');
		}

		// Update workout type filter
		if (filters.workoutTypes.length > 0) {
			url.searchParams.set('workout_type', filters.workoutTypes.join(','));
		} else {
			url.searchParams.delete('workout_type');
		}

		// Update sport category filter
		if (filters.sportCategories.length > 0) {
			url.searchParams.set('sport_category', filters.sportCategories.join(','));
		} else {
			url.searchParams.delete('sport_category');
		}

		// Update show notes preference
		if (filters.showNotes) {
			url.searchParams.set('show_notes', 'true');
		} else {
			url.searchParams.delete('show_notes');
		}

		goto(url, { replaceState: true, keepFocus: true });
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

	const handleActivityDeleted = () => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		invalidate('app:activities');
	};

	const handleActivityUpdated = (activityId: string) => {
		invalidate('app:activities');
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="@container mx-2 mt-5 sm:mx-auto">
	<!-- View Toggle -->
	<div class="mb-4 flex flex-col justify-between gap-2 @sm:flex-row @sm:items-center">
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
		<div class="flex h-[100vh] flex-row gap-2 overflow-hidden">
			<div class="grow basis-0 overflow-y-auto">
				<ActivitiesList
					activityList={sorted_activities}
					{initialRpe}
					{initialWorkoutTypes}
					{initialSportCategories}
					{initialShowNotes}
					onFiltersChange={handleFilterChange}
					onActivitySelected={handleActivitySelected}
					selectedActivity={selectedActivityId}
				/>
			</div>
			{#if selectedActivityPromise && screenWidth >= 700}
				<div
					class="relative w-full grow basis-0 overflow-auto rounded-box bg-base-100 pt-4 shadow-md"
				>
					{#await selectedActivityPromise}
						<div class="flex items-center justify-center rounded-box bg-base-100 p-8 shadow-md">
							<span class="loading loading-lg loading-spinner"></span>
						</div>
					{:then selectedActivity}
						{#if selectedActivity}
							<button onclick={() => handleActivitySelected(null)} class="absolute right-3"
								>X</button
							>
							<ActivityDetails
								activity={selectedActivity}
								onActivityUpdated={() => handleActivityUpdated(selectedActivity.id)}
								onActivityDeleted={handleActivityDeleted}
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
			activityList={sorted_activities}
			{currentMonth}
			onMonthChange={handleMonthChange}
		/>
	{/if}
</div>
