<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import type { TrainingMetric } from '$lib/api/training';
	import { metricValuesDisplayFormat } from '$lib/trainingMetric';
	import TrainingMetricsChartLine from './TrainingMetricsChartLine.svelte';
	import { none, some, type Option } from '$lib/Options';

	let {
		metrics,
		height,
		onUpdate,
		timeDomain = none()
	}: {
		metrics: TrainingMetric[];
		height: number;
		onUpdate: () => void;
		onDelete: () => void;
		timeDomain?: Option<{ start: string; end: string | null }>;
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
			let scope: 'global' | 'local' = metric.scope.type === 'global' ? 'global' : 'local';

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
				showGroup: metric.group_by !== null,
				scope,
				initialMetric: metric,
				summary: metric.summary
			};
		})
	);
</script>

<div class="flex flex-col items-center gap-0">
	{#each metricProps as metric, idx (metric.id)}
		<div class="flex w-full flex-col gap-0" bind:clientWidth={chartWidth}>
			<TrainingMetricTitle metric={metric.initialMetric} {onUpdate} />

			{#if metric.values.length > 0}
				{#if metric.granularity !== null}
					<TrainingMetricsChartStacked
						{height}
						width={chartWidth}
						values={metric.values}
						unit={metric.unit}
						granularity={metric.granularity}
						format={metricValuesDisplayFormat(metric)}
						showGroup={metric.showGroup}
						groupBy={metric.groupBy}
						stacked={metric.aggregate === 'Sum' || metric.aggregate === 'NumberOfActivities'}
						average={'average' in metric.summary ? some(metric.summary.average) : none()}
					/>
				{:else}
					<TrainingMetricsChartLine
						height={300}
						width={chartWidth}
						values={metric.values}
						unit={metric.unit}
						format={metricValuesDisplayFormat(metric)}
						average={'average' in metric.summary ? some(metric.summary.average) : none()}
						{timeDomain}
					/>
				{/if}
			{:else}
				<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
			{/if}

			{#if idx !== metricProps.length - 1}
				<div class="divider"></div>
			{/if}
		</div>
	{:else}
		<div class="p-3 text-center text-sm tracking-wide italic opacity-60">No training metrics</div>
	{/each}
</div>
