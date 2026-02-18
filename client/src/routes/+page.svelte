<script lang="ts">
	import { goto } from '$app/navigation';
	import { invalidate } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsCarousel from '$components/organisms/TrainingMetricsCarousel.svelte';
	import TrainingPeriodCard from '$components/molecules/TrainingPeriodCard.svelte';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';
	import {
		fetchActivityDetails,
		type ActivityDetails as ActivityDetailsType
	} from '$lib/api/activities';
	import TrainingMetricsList from '$components/organisms/TrainingMetricsList.svelte';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);
	let chartHeight = $derived(Math.max(250, Math.min(300, chartWidth * 0.6)));
	let selectedActivityPromise: Promise<ActivityDetailsType | null> | null = $state(null);
	let selectedActivityId: string | null = $state(null);
	let screenWidth = $state(0);

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
		selectedActivityId = activityId;
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

<div class="homepage_container">
	{#await data.trainingPeriods}
		<div class="item flex flex-col items-center rounded-box bg-base-100 p-4 pt-6 shadow-md">
			<div class="loading loading-bars"></div>
		</div>
	{:then periods}
		{#if periods.length > 0}
			<div
				class={`item periods rounded-box bg-base-100 shadow-md ${selectedActivityId === null ? 'flex' : 'hidden!'}`}
			>
				<div class="p-4">
					<h2 class="mb-3 text-lg font-semibold">Ongoing Training Periods</h2>
					<div class="flex flex-col gap-2">
						{#each periods as period}
							<TrainingPeriodCard {period} />
						{/each}
					</div>
				</div>
			</div>
		{/if}
	{/await}

	<div
		bind:clientWidth={chartWidth}
		class={`item metric_chart rounded-box bg-base-100 shadow-md  ${selectedActivityId === null ? 'flex' : 'hidden!'}`}
	>
		<h2 class="px-4 pt-4 text-lg font-semibold">Training metrics</h2>
		{#await data.metrics}
			<div class="item flex flex-col items-center rounded-box bg-base-100 p-4 pt-6 shadow-md">
				<div class="loading loading-bars"></div>
			</div>
		{:then metrics}
			{#if screenWidth < 700}
				<TrainingMetricsCarousel
					{metrics}
					height={chartHeight}
					onUpdate={() => invalidate(`app:activities`)}
					onDelete={() => invalidate(`app:activities`)}
				/>
			{:else}
				<TrainingMetricsList
					{metrics}
					height={chartHeight}
					onUpdate={() => invalidate(`app:activities`)}
					onDelete={() => invalidate(`app:activities`)}
				/>
			{/if}
		{/await}
	</div>

	<div class="item history">
		<div>
			{#await data.activitiesWithNotes}
				<div class="item flex flex-col items-center rounded-box bg-base-100 p-4 pt-6 shadow-md">
					<div class="loading loading-bars"></div>
				</div>
			{:then [activities, notes]}
				<PastActivitiesList
					activityList={activities}
					trainingNotes={notes}
					moreCallback={moreActivitiesCallback}
					onNoteSave={handleNoteSave}
					onNoteDelete={handleNoteDelete}
					onActivityClick={handleActivityClick}
					{selectedActivityId}
				/>
			{/await}
		</div>
	</div>

	{#if selectedActivityPromise}
		<div class={`item activity_details rounded-box bg-base-100 pt-4 shadow-md `}>
			{#await selectedActivityPromise}
				<div class="flex w-full items-center justify-center rounded-box bg-base-100 p-8 shadow-md">
					<span class="loading loading-lg loading-spinner"></span>
				</div>
			{:then activityDetails}
				{#if activityDetails}
					<div class="relative w-full">
						<button
							onclick={() => {
								selectedActivityId = null;
								selectedActivityPromise = null;
							}}
							class="absolute right-3">X</button
						>
						<ActivityDetails
							activity={activityDetails}
							onActivityUpdated={() => {
								invalidate('app:activities');
							}}
							onActivityDeleted={() => {
								invalidate('app:activities');
								selectedActivityId = null;
								selectedActivityPromise = null;
							}}
							compact={true}
						/>
					</div>
				{/if}
			{/await}
		</div>
	{/if}
</div>

<style>
	.homepage_container {
		width: 100%;
		display: flex;
		flex-direction: column;
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

	.history {
		display: flex;
		flex-direction: column;
		gap: calc(var(--spacing) * 5);
	}

	.metric_chart {
		display: flex;
		flex-direction: column;
	}

	.activity_details {
		display: none;
	}

	@media (min-width: 700px) {
		.homepage_container {
			display: grid;
			grid-template-columns: repeat(2, minmax(0, 1fr));
			grid-template-rows: auto 1fr;
			align-items: start;
			margin-top: calc(var(--spacing) * 5);
		}
		.history {
			grid-column: 1;
			grid-row: 1 / span 2;
			padding-right: calc(var(--spacing) * 2);
		}

		.activity_details {
			display: flex;
			grid-row: 1 / span 2;
			grid-column: 2 / span 1;
		}

		.periods {
			grid-row: 1 / span 1;
			grid-column: 2 / span 1;
		}

		.metric_chart {
			grid-row: 2 / span 1;
			grid-column: 2 / span 1;
		}
	}
</style>
