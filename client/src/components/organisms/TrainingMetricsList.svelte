<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import type { MetricsListItemGrouped } from '$lib/api/training';

	let {
		metrics,
		height
	}: {
		metrics: MetricsListItemGrouped[];
		height: number;
	} = $props();

	let chartWidth: number = $state(300);

	let metricProps = $derived(
		metrics.map((metric) => {
			let values = [];
			for (const [group, time_values] of Object.entries(metric.values)) {
				for (const [dt, value] of Object.entries(time_values)) {
					values.push({ time: dt, group, value });
				}
			}
			return {
				id: metric.id,
				name: metric.name,
				values: values,
				metric: metric.metric,
				granularity: metric.granularity,
				aggregate: metric.aggregate,
				sports: metric.sports,
				groupBy: metric.group_by,
				unit: metric.unit,
				showGroup: metric.group_by !== null
			};
		})
	);
</script>

<div class="flex flex-col items-center gap-0">
	{#each metricProps as metric, idx (metric.id)}
		<div class="flex w-full flex-col gap-0" bind:clientWidth={chartWidth}>
			<TrainingMetricTitle
				name={metric.name}
				granularity={metric.granularity}
				aggregate={metric.aggregate}
				metric={metric.metric}
				sports={metric.sports}
				groupBy={metric.groupBy}
				isFavorite={false}
			/>

			<TrainingMetricsChartStacked
				{height}
				width={chartWidth}
				values={metric.values}
				unit={metric.unit}
				granularity={metric.granularity}
				format={metric.unit === 's' ? 'duration' : 'number'}
				showGroup={metric.showGroup}
				groupBy={metric.groupBy}
			/>

			{#if idx !== metricProps.length - 1}
				<div class="divider"></div>
			{/if}
		</div>
	{/each}
</div>
