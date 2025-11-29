<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import type { MetricsListItemGrouped } from '$lib/api/training';
	import TrainingMetricMenu from '$components/molecules/TrainingMetricMenu.svelte';

	let {
		metrics,
		height,
		onDelete,
		onUpdate
	}: {
		metrics: MetricsListItemGrouped[];
		height: number;
		onUpdate: () => void;
		onDelete: () => void;
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
			<div class="flex flex-row justify-center">
				<TrainingMetricTitle
					name={metric.name}
					granularity={metric.granularity}
					aggregate={metric.aggregate}
					metric={metric.metric}
					sports={metric.sports}
					groupBy={metric.groupBy}
				/>
				<TrainingMetricMenu
					metric={{
						id: metric.id,
						name: metric.name || ''
					}}
					{onDelete}
					{onUpdate}
				/>
			</div>

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
	{:else}
		<div class="p-3 text-center text-sm italic opacity-90">No training metrics</div>
	{/each}
</div>
