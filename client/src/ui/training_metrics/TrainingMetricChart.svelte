<script lang="ts">
	import { getMetricGroupBy, type TrainingMetric } from '$lib/api';
	import { isSome, none, some, type Option } from '$lib/Options';
	import ScatterChart from './internal/charts/Scatter.svelte';
	import BarChart from './internal/charts/Bar.svelte';
	import StackedArea from './internal/charts/StackedArea.svelte';
	import Polyline from './internal/charts/Polyline.svelte';
	import { type DisplayMode } from './internal/charts';
	import type { TimeDomain } from '$ui/training_metrics';

	export interface ChartHandle {
		getYMax(): number;
		getYMin(): number;
	}

	export interface HoveredBin {
		bin: number;
		group?: string;
	}

	let {
		metric,
		width,
		height = 300,
		timeDomain = none(),
		displayMode = 'absolute',
		yMax = none(),
		syncHoveredBin = $bindable(none())
	}: {
		metric: TrainingMetric;
		width: number;
		height?: number;
		timeDomain?: TimeDomain;
		displayMode?: DisplayMode;
		yMax?: Option<number>;
		syncHoveredBin?: Option<HoveredBin>;
	} = $props();

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};
	let groupBy = $derived(getMetricGroupBy(metric));
	let format = $derived(previewFormat(metric.unit));
	let showGroup = $derived(isSome(groupBy));
	let stacked = $derived(metric.aggregate === 'Sum');
	let average: Option<number> = $derived(
		'average' in metric.summary ? some(metric.summary.average) : none()
	);
	let target: Option<number> = $derived(
		metric.target === null ? none() : some(metric.target.value)
	);
	let values = $derived(metric.values);

	let chartRef = $state<ChartHandle>();
	export function getYMax(): number {
		return chartRef?.getYMax() ?? 0;
	}
	export function getYMin(): number {
		return chartRef?.getYMin() ?? 0;
	}

	({ getYMax, getYMin }) satisfies ChartHandle;
</script>

{#if Object.entries(metric.values).length > 0}
	{#if metric.source.type === 'activity'}
		{#if metric.granularity !== null}
			<BarChart
				bind:this={chartRef}
				{height}
				{width}
				{values}
				unit={metric.unit}
				granularity={metric.granularity}
				{format}
				{showGroup}
				{groupBy}
				{stacked}
				{average}
				{target}
				{timeDomain}
				{displayMode}
				yMaxValue={yMax}
				bind:syncHoveredBin
			/>
		{:else}
			<ScatterChart
				bind:this={chartRef}
				{height}
				{width}
				values={metric.values}
				unit={metric.unit}
				format={previewFormat(metric.unit)}
				average={'average' in metric.summary ? some(metric.summary.average) : none()}
				target={metric.target === null ? none() : some(metric.target.value)}
				{timeDomain}
				yInterceptZero={!metric.source.metric.metric.includes('HeartRate')}
				{displayMode}
				yMaxValue={yMax}
			/>
		{/if}
	{:else if metric.granularity !== null}
		{#if metric.source.type === 'weightAndNutrition' && (metric.source.metric === 'BodyComposition' || metric.source.metric === 'Macros')}
			<StackedArea
				bind:this={chartRef}
				data={metric.values}
				unit={metric.unit}
				format="number"
				average={'average' in metric.summary ? some(metric.summary.average) : none()}
				target={metric.target === null ? none() : some(metric.target.value)}
				{width}
				{height}
				{timeDomain}
				granularity={metric.granularity}
				{displayMode}
				yMaxValue={yMax}
				bind:syncHoveredBin
			/>
		{:else}
			<Polyline
				bind:this={chartRef}
				data={values}
				unit={metric.unit}
				format="number"
				average={'average' in metric.summary ? some(metric.summary.average) : none()}
				target={metric.target === null ? none() : some(metric.target.value)}
				{width}
				{height}
				yMaxValue={metric.source.type === 'hooperIndex' ? some(10) : yMax}
				{timeDomain}
				granularity={metric.granularity}
				yInterceptZero={metric.source.metric !== 'TotalWeight'}
				{displayMode}
				bind:syncHoveredBin
			/>
		{/if}
	{/if}
{:else}
	<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
{/if}
