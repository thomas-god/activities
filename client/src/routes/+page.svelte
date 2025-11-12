<script lang="ts">
	import { goto } from '$app/navigation';
	import { invalidate } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
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

	let topMetric = $derived.by(() => {
		// Find favorite metric ID from preferences
		const favoriteMetricPref = data.preferences.find((p) => p.key === 'favorite_metric');
		const favoriteMetricId = favoriteMetricPref?.value;

		// Try to find the favorite metric, otherwise use the first one
		let metric = favoriteMetricId ? data.metrics.find((m) => m.id === favoriteMetricId) : undefined;

		if (metric === undefined) {
			metric = data.metrics.at(0);
		}

		if (metric === undefined) {
			return undefined;
		}

		let values = [];
		for (const [group, time_values] of Object.entries(metric.values)) {
			for (const [dt, value] of Object.entries(time_values)) {
				values.push({ time: dt, group, value });
			}
		}

		return {
			id: metric.id,
			values: values,
			metric: metric.metric,
			granularity: metric.granularity,
			aggregate: metric.aggregate,
			sports: metric.sports,
			groupBy: metric.group_by,
			unit: metric.unit,
			showGroup: metric.group_by !== null
		};
	});

	const moreActivitiesCallback = () => {
		goto('/history');
	};

	const handleNoteSave = async (noteId: string, content: string, date: string) => {
		await updateTrainingNote(noteId, content, date);
		await invalidate('app:activities');
	};

	const handleNoteDelete = async (noteId: string) => {
		await deleteTrainingNote(noteId);
		await invalidate('app:activities');
	};
</script>

{#if topMetric}
	<div
		bind:clientWidth={chartWidth}
		class="mx-2 mt-5 rounded-box bg-base-100 pb-2 shadow-md sm:mx-auto"
	>
		<div class="mx-3 pt-4 text-center">
			<TrainingMetricTitle
				granularity={topMetric.granularity}
				aggregate={topMetric.aggregate}
				metric={topMetric.metric}
				sports={topMetric.sports}
				groupBy={topMetric.groupBy}
			/>
		</div>
		<TrainingMetricsChartStacked
			height={chartHeight}
			width={chartWidth}
			values={topMetric.values}
			unit={topMetric.unit}
			granularity={topMetric.granularity}
			format={topMetric.unit === 's' ? 'duration' : 'number'}
			showGroup={topMetric.showGroup}
			groupBy={topMetric.groupBy}
		/>
	</div>
{/if}

{#if ongoingPeriods.length > 0}
	<div class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto">
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

<div class="mx-2 mt-5 sm:mx-auto">
	<PastActivitiesList
		activityList={sorted_activities}
		trainingNotes={data.trainingNotes}
		moreCallback={moreActivitiesCallback}
		onNoteSave={handleNoteSave}
		onNoteDelete={handleNoteDelete}
	/>
</div>
