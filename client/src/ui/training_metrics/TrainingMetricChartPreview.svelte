<script lang="ts">
	import type { TrainingMetric } from '$lib/api';
	import { none, some, type Option } from '$lib/Options';
	import TrainingMetricChartLine from './TrainingMetricChartLine.svelte';
	import TrainingMetricChartStacked from './TrainingMetricChartStacked.svelte';

	let {
		metric,
		width,
		timeDomain = none()
	}: {
		metric: TrainingMetric;
		width: number;
		timeDomain?: Option<{ start: string; end: string | null }>;
	} = $props();

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};
</script>

{#if metric.granularity !== null}
	<TrainingMetricChartStacked
		height={300}
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
		height={300}
		{width}
		values={metric.values}
		unit={metric.unit}
		format={previewFormat(metric.unit)}
		average={'average' in metric.summary ? some(metric.summary.average) : none()}
		target={metric.target === null ? none() : some(metric.target.value)}
		{timeDomain}
	/>
{/if}
