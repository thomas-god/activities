<script lang="ts">
	import { goto } from '$app/navigation';
	import { invalidate } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsCarousel from '$components/organisms/TrainingMetricsCarousel.svelte';
	import TrainingPeriodCard from '$components/molecules/TrainingPeriodCard.svelte';
	import { dayjs } from '$lib/duration';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';
	import {
		fetchActivityDetails,
		type ActivityDetails as ActivityDetailsType
	} from '$lib/api/activities';
	import TrainingMetricsList from '$components/organisms/TrainingMetricsList.svelte';

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

	let sortedMetrics = $derived.by(() => {
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
		if (screenWidth < 700) {
			goto(`/activity/${activityId}`);
			return;
		}

		// On desktop, load and show activity details in right column
		// selectedActivityId = activityId;
		// selectedActivityPromise = fetchActivityDetails(fetch, activityId);
		// TODO: load activity quick preview
		goto(`/activity/${activityId}`);
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="homepage_container">
	<div
		bind:clientWidth={chartWidth}
		class="item metric_chart rounded-box bg-base-100 p-4 shadow-md"
	>
		<h2 class=" text-lg font-semibold">Training metrics</h2>
		{#if screenWidth < 700}
			<TrainingMetricsCarousel metrics={sortedMetrics} height={chartHeight} {favoriteMetricId} />
		{:else}
			<TrainingMetricsList metrics={sortedMetrics} height={chartHeight} />
		{/if}
	</div>

	{#if ongoingPeriods.length > 0}
		<div class="item periods rounded-box bg-base-100 shadow-md">
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

	<div class="item history">
		<div>
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
		display: grid;
		grid-template-rows: auto;
		align-items: center;
		gap: calc(var(--spacing) * 3);
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
		grid-row: 3 / span 1;
		display: flex;
		flex-direction: column;
		gap: calc(var(--spacing) * 5);
	}

	.item.metric_chart {
		grid-row: 2 / span 1;
	}

	.item.periods {
		grid-row: 1 / span 1;
	}

	@media (min-width: 700px) {
		.homepage_container {
			display: grid;
			grid-template-columns: 1fr 1fr;
			grid-template-rows: auto 1fr;
			align-items: start;
			margin-top: calc(var(--spacing) * 5);
		}
		.item.history {
			grid-column: 1;
			grid-row: 1 / span 2;
			padding-right: calc(var(--spacing) * 2);
		}

		.item.periods {
			grid-row: 1 / span 1;
			grid-column: 2 / span 1;
		}

		.item.metric_chart {
			grid-row: 2 / span 1;
			grid-column: 2 / span 1;
		}
	}
</style>
