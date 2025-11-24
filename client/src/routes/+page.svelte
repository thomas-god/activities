<script lang="ts">
	import { goto } from '$app/navigation';
	import { invalidate } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsCarousel from '$components/organisms/TrainingMetricsCarousel.svelte';
	import TrainingPeriodCard from '$components/molecules/TrainingPeriodCard.svelte';
	import { dayjs } from '$lib/duration';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import {
		fetchActivityDetails,
		type ActivityDetails as ActivityDetailsType
	} from '$lib/api/activities';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);
	let chartHeight = $derived(Math.max(150, Math.min(300, chartWidth * 0.6)));
	let selectedActivityPromise: Promise<ActivityDetailsType | null> | null = $state(null);
	let selectedActivityId: string | null = $state(null);
	let screenWidth = $state(0);

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	// Filter ongoing training periods (no end date or end date >= today)
	let ongoingPeriods = $derived.by(() => {
		const today = dayjs().startOf('day');
		return data.trainingPeriods.filter((period) => {
			if (period.end === null) return true;
			return dayjs(period.end).isAfter(today) || dayjs(period.end).isSame(today);
		});
	});

	let favoriteMetricId = $derived.by(() => {
		const favoriteMetricPref = data.preferences.find((p) => p.key === 'favorite_metric');
		return favoriteMetricPref?.value;
	});

	let metricsForCarousel = $derived.by(() => {
		// Sort to put favorite first if it exists
		const sortedMetrics = [...data.metrics];
		if (favoriteMetricId) {
			sortedMetrics.sort((a, b) => {
				if (a.id === favoriteMetricId) return -1;
				if (b.id === favoriteMetricId) return 1;
				return 0;
			});
		}
		return sortedMetrics;
	});

	const moreActivitiesCallback = () => {
		goto('/history');
	};

	const handleNoteSave = async (noteId: string, content: string, date: string) => {
		await updateTrainingNote(noteId, content, date);
		await invalidate('app:training-notes');
	};

	const handleNoteDelete = async (noteId: string) => {
		await deleteTrainingNote(noteId);
		await invalidate('app:training-notes');
	};

	const handleActivityClick = async (activityId: string) => {
		// On mobile, navigate to activity page
		if (screenWidth < 900) {
			goto(`/activity/${activityId}`);
			return;
		}

		// On desktop, load and show activity details in right column
		selectedActivityId = activityId;
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};

	const handleActivityDeleted = () => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		invalidate('app:activities');
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="homepage_container">
	<div bind:clientWidth={chartWidth} class="item metric_chart">
		<div class="">
			{#if metricsForCarousel.length > 0}
				<details
					class="collapse-arrow collapse rounded-box border border-base-300 bg-base-100 shadow-md"
					open
				>
					<summary class="collapse-title text-lg font-semibold">Training Metrics</summary>
					<div class="collapse-content">
						<TrainingMetricsCarousel
							metrics={metricsForCarousel}
							width={chartWidth}
							height={chartHeight}
							{favoriteMetricId}
						/>
					</div>
				</details>
			{/if}

			<div class="activity_details mt-3">
				<div class="divider"></div>
				{#if selectedActivityPromise}
					{#await selectedActivityPromise}
						<div class="flex items-center justify-center rounded-box bg-base-100 p-8 shadow-md">
							<span class="loading loading-lg loading-spinner"></span>
						</div>
					{:then selectedActivity}
						{#if selectedActivity}
							<div>
								<ActivityDetails
									activity={selectedActivity}
									onActivityUpdated={() => {
										// TODO: handle update
									}}
									onActivityDeleted={handleActivityDeleted}
								/>
							</div>
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
				{:else}
					<div
						class="mt-5 flex items-center justify-center rounded-box bg-base-100 p-8 text-base-content/60 shadow-md"
					>
						Select an activity to view details
					</div>
				{/if}
			</div>
		</div>
	</div>

	<div class="item history">
		{#if ongoingPeriods.length > 0}
			<div class="rounded-box bg-base-100 shadow-md">
				<div class="p-4">
					<h2 class="mb-3 text-lg font-semibold">Ongoing Training Periods</h2>
					<div class="flex flex-col gap-2">
						{#each ongoingPeriods as period}
							<TrainingPeriodCard {period} />
						{/each}
					</div>
				</div>
			</div>
		{/if}

		<div class="">
			<PastActivitiesList
				activityList={sorted_activities}
				trainingNotes={data.trainingNotes}
				moreCallback={moreActivitiesCallback}
				onNoteSave={handleNoteSave}
				onNoteDelete={handleNoteDelete}
				onActivityClick={handleActivityClick}
				{selectedActivityId}
			/>
		</div>
	</div>
</div>

<style>
	.homepage_container {
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: calc(var(--spacing) * 5);
		margin-top: calc(var(--spacing) * 5);
		padding-inline: calc(var(--spacing) * 1);

		@media (min-width: 400px) {
			padding-inline: calc(var(--spacing) * 2);
		}
	}

	.item {
		width: 100%;
	}

	.item.history {
		display: flex;
		flex-direction: column;
		gap: calc(var(--spacing) * 5);
	}

	.item.metric_chart {
		& .collapse-content {
			padding-left: 0rem;
			padding-right: 0rem;
		}
	}

	.activity_details {
		display: none;
	}

	@media (min-width: 900px) {
		.homepage_container {
			display: grid;
			grid-template-columns: minmax(20rem, 32rem) minmax(20rem, 800px);
			align-items: start;
			height: calc(100vh - calc(var(--spacing) * 5) - 60px);
			margin-top: calc(var(--spacing) * 5);
			overflow: hidden;
		}

		.item.metric_chart {
			display: flex;
			flex-direction: column;
			gap: calc(var(--spacing) * 5);
			grid-column: 2;
			grid-row: 1;
			height: 100%;
			overflow-y: auto;
			padding-right: calc(var(--spacing) * 2);

			& .collapse-content {
				padding-left: 1rem;
				padding-right: 1rem;
			}
		}

		.item.history {
			grid-column: 1;
			grid-row: 1 / span 2;
			height: 100%;
			overflow-y: auto;
			padding-right: calc(var(--spacing) * 2);
		}

		.activity_details {
			display: block;
		}
	}
</style>
