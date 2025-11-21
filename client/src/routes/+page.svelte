<script lang="ts">
	import { goto } from '$app/navigation';
	import { invalidate } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsCarousel from '$components/organisms/TrainingMetricsCarousel.svelte';
	import TrainingPeriodCard from '$components/molecules/TrainingPeriodCard.svelte';
	import { dayjs } from '$lib/duration';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);
	let chartHeight = $derived(Math.max(150, Math.min(300, chartWidth * 0.6)));

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
</script>

<div class="homepage_container">
	{#if metricsForCarousel.length > 0}
		<div
			bind:clientWidth={chartWidth}
			class="item metric_chart rounded-box bg-base-100 pb-2 shadow-md"
		>
			<TrainingMetricsCarousel
				metrics={metricsForCarousel}
				width={chartWidth}
				height={chartHeight}
				{favoriteMetricId}
			/>
		</div>
	{/if}

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

	@media (min-width: 900px) {
		.homepage_container {
			display: grid;
			grid-template-columns: minmax(20rem, 32rem) minmax(20rem, 800px);
			align-items: start;
		}

		.item.metric_chart {
			grid-column: 2;
		}

		.item.history {
			grid-column: 1;
			grid-row: 1;
		}
	}
</style>
