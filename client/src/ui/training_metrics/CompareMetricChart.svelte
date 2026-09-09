<script lang="ts">
	import type { TrainingMetric } from '$lib/api';
	import { metricValuesDisplayFormat } from '$lib/trainingMetric';
	import { none, some } from '$lib/Options';
	import CompareMetricChartBars from './internal/CompareMetricChartBars.svelte';

	let {
		metric,
		anchor,
		bucketDomain,
		yDomain,
		width,
		height,
		hovered = $bindable(null)
	}: {
		metric: TrainingMetric;
		/** Date the compared charts align their buckets to. */
		anchor: string;
		/** Shared x-axis domain of the comparison (union of both periods' bucket offsets). */
		bucketDomain: number[];
		yDomain: number[];
		width: number;
		height: number;
		/** Bucket offset currently hovered, shared between the compared charts. */
		hovered?: number | null;
	} = $props();

	let average = $derived(
		'average' in metric.summary ? some(metric.summary.average) : none<number>()
	);
	let target = $derived(metric.target === null ? none<number>() : some(metric.target.value));

	let yScale = $derived.by(() => {
		if (metric.metric === 'Elevation' && metric.unit === 'km') {
			return { factor: 1000, unit: 'm' };
		}
		return { factor: 1, unit: metric.unit };
	});
</script>

{#if metric.granularity === null}
	<div class="p-3 text-center text-sm tracking-wide italic opacity-60">
		This metric cannot be compared
	</div>
{:else if Object.entries(metric.values).length === 0}
	<div class="p-3 text-center text-sm tracking-wide italic opacity-60">No values found</div>
{:else}
	<CompareMetricChartBars
		values={metric.values}
		{anchor}
		{bucketDomain}
		{yDomain}
		yScaleFactor={yScale.factor}
		granularity={metric.granularity}
		unit={yScale.unit}
		format={metricValuesDisplayFormat(metric)}
		groupBy={metric.group_by}
		showGroup={metric.group_by !== null}
		stacked={metric.aggregate === 'Sum'}
		{average}
		{target}
		{width}
		{height}
		bind:hovered
	/>
{/if}
