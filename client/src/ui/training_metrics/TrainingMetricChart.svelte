<script lang="ts">
	import { getMetricGroupBy, type TrainingMetric } from '$lib/api';
	import { asOption, isSome, none, some, type Option } from '$lib/Options';
	import ScatterChart from './internal/charts/Scatter.svelte';
	import BarChart from './internal/charts/Bar.svelte';
	import StackedArea from './internal/charts/StackedArea.svelte';
	import Polyline from './internal/charts/Polyline.svelte';
	import { type TimeDomain } from './internal/charts';

	let {
		metric,
		width,
		height = 300,
		timeDomain = none()
	}: {
		metric: TrainingMetric;
		width: number;
		height?: number;
		timeDomain?: TimeDomain;
	} = $props();

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};
	let groupBy = $derived(getMetricGroupBy(metric));
	let granularity = $derived(asOption(metric.granularity));
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
</script>

{#if Object.entries(metric.values).length > 0}
	{#if metric.source.type === 'activity'}
		{#if metric.granularity !== null}
			<BarChart
				{height}
				{width}
				{values}
				unit={metric.unit}
				{granularity}
				{format}
				{showGroup}
				{groupBy}
				{stacked}
				{average}
				{target}
				{timeDomain}
			/>
		{:else}
			<ScatterChart
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
		{#if metric.source.type === 'weightAndNutrition' && (metric.source.metric === 'BodyComposition' || metric.source.metric === 'Macros')}
			<StackedArea
				data={metric.values}
				unit={metric.unit}
				format="number"
				average={'average' in metric.summary ? some(metric.summary.average) : none()}
				target={metric.target === null ? none() : some(metric.target.value)}
				{width}
				{height}
				yMaxValue={none()}
				{timeDomain}
				{granularity}
			/>
		{:else}
			<Polyline
				data={values}
				unit={metric.unit}
				format="number"
				average={'average' in metric.summary ? some(metric.summary.average) : none()}
				target={metric.target === null ? none() : some(metric.target.value)}
				{width}
				{height}
				yMaxValue={some(10)}
				{timeDomain}
				{granularity}
			/>
		{/if}
	{/if}
{:else}
	<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
{/if}
