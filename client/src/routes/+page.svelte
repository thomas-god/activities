<script lang="ts">
	import { goto } from '$app/navigation';
	import PastActivitiesList from '../organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsChart from '../organisms/TrainingMetricsChart.svelte';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

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

		return {
			values: values,
			title: `${metric.metric} (${metric.granularity})`,
			unit: metric.unit,
			granularity: metric.granularity
		};
	});

	const moreActivitiesCallback = () => {
		goto('/history');
	};
</script>

{#if topMetric}
	<div bind:clientWidth={chartWidth} class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto">
		<p class="mx-2 pt-4">{topMetric.title}</p>
		<TrainingMetricsChart
			height={300}
			width={chartWidth}
			values={topMetric.values}
			unit={topMetric.unit}
			granularity={topMetric.granularity}
			format={topMetric.unit === 's' ? 'duration' : 'number'}
		/>
	</div>
{/if}

<div class=" mx-2 mt-5 sm:mx-auto">
	<PastActivitiesList activityList={sorted_activities} moreCallback={moreActivitiesCallback} />
</div>
