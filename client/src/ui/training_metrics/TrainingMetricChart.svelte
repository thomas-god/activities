<script lang="ts">
	import { getMetricGroupBy, type TrainingMetric } from '#lib/api/index.js';
	import { isSome, none, some, type Option } from '#lib/Options.js';
	import ScatterChart from './internal/charts/Scatter.svelte';
	import BarChart from './internal/charts/Bar.svelte';
	import StackedArea from './internal/charts/StackedArea.svelte';
	import Polyline from './internal/charts/Polyline.svelte';
	import { type DisplayMode } from './internal/charts';
	import type { TimeDomain } from '#ui/training_metrics';
	import DurationCurve from '#ui/activity/DurationCurve.svelte';

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

	// Durations (in seconds) of the 12 bins of a duration curve, matching the
	// server's DURATION_CURVE_DURATIONS_SECOND and PowerCurve's FIXED_DURATIONS.
	const DURATION_CURVE_DURATIONS = [5, 10, 30, 60, 120, 300, 600, 1200, 1800, 3600, 7200, 18000];

	// Duration curve metrics are not grouped: values are keyed by duration (in
	// seconds) under the no-group bucket. Rebuild the 12-slots array expected by
	// the power curve component, in the fixed durations order.
	let durationCurveValues = $derived.by(() => {
		if (metric.source.type !== 'durationCurve') return [];
		const granules = Object.values(metric.values).at(0) ?? {};
		return DURATION_CURVE_DURATIONS.map((duration) => granules[String(duration)] ?? null);
	});

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
	{:else if metric.source.type === 'durationCurve'}
		{@const curve_type = metric.source.metric === 'Running' ? 'pace' : 'power'}
		<DurationCurve kind={curve_type} curveValues={durationCurveValues} {width} {height} />
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
