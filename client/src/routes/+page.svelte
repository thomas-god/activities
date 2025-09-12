<script lang="ts">
	import { invalidate } from '$app/navigation';
	import ActivityList from '../organisms/ActivityList.svelte';
	import ActivitiesUploader from '../organisms/ActivitiesUploader.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsChart from '../organisms/TrainingMetricsChart.svelte';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	const activitiesUploadedCallback = () => {
		invalidate('app:activities');
	};

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	let topMetric = $derived.by(() => {
		let metric = data.metrics.at(0);
		if (metric === undefined) {
			return undefined;
		}
		let values = [];
		for (const dt in metric.values) {
			values.push({ time: dt, value: metric.values[dt] });
		}

		return { values: values, title: `${metric.metric} (${metric.granularity})` };
	});
</script>

<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
	<ActivitiesUploader {activitiesUploadedCallback} />
</div>

{#if topMetric}
	<div
		bind:clientWidth={chartWidth}
		class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto sm:w-2xl"
	>
		<p class="text-center">{topMetric.title}</p>
		<TrainingMetricsChart
			height={250}
			width={chartWidth}
			values={topMetric.values}
			title={topMetric.title}
		/>
	</div>
{/if}

<div class="mx-2 mt-5 sm:mx-auto sm:w-lg">
	<ActivityList activityList={sorted_activities} />
</div>
