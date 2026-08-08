<script lang="ts">
	import type { TrainingMetric } from '$lib/api';
	import { none, some, type Option } from '$lib/Options';
	import TrainingMetricChartLine from './internal/TrainingMetricChartLine.svelte';
	import TrainingMetricChartStacked from './internal/TrainingMetricChartStacked.svelte';

	let {
		metric,
		width,
		height = 300,
		timeDomain = none()
	}: {
		metric: TrainingMetric;
		width: number;
		height?: number;
		timeDomain?: Option<{ start: string; end: string | null }>;
	} = $props();

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};
</script>

{#if Object.entries(metric.values).length > 0}
	{#if metric.granularity !== null}
		<TrainingMetricChartStacked
			{height}
			{width}
			values={metric.values}
			unit={metric.unit}
			granularity={metric.granularity}
			format={previewFormat(metric.unit)}
			showGroup={metric.group_by !== null}
			groupBy={metric.group_by}
			stacked={metric.aggregate === 'Sum'}
			average={'average' in metric.summary ? some(metric.summary.average) : none()}
			target={metric.target === null ? none() : some(metric.target.value)}
		/>
	{:else}
		<TrainingMetricChartLine
			{height}
			{width}
			values={metric.values}
			unit={metric.unit}
			format={previewFormat(metric.unit)}
			average={'average' in metric.summary ? some(metric.summary.average) : none()}
			target={metric.target === null ? none() : some(metric.target.value)}
			{timeDomain}
		/>
	{/if}
{:else}
	<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
{/if}
