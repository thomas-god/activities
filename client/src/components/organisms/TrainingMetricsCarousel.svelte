<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import type { TrainingMetric } from '$lib/api/training';
	import { metricValuesDisplayFormat } from '$lib/trainingMetric';
	import TrainingMetricsChartLine from './TrainingMetricsChartLine.svelte';
	import { none, some, type Option } from '$lib/Options';
	import TrainingMetricMenu from '$components/molecules/TrainingMetricMenu.svelte';

	let {
		metrics,
		height,
		onMetricUpdate,
		initialIndex = 0,
		timeDomain = none()
	}: {
		metrics: TrainingMetric[];
		height: number;
		onMetricUpdate: () => void;
		initialIndex?: number;
		timeDomain?: Option<{ start: string; end: string | null }>;
	} = $props();

	let chartWidth: number = $state(300);
	let currentIndex = $derived(initialIndex);
	let currentMetric = $derived(metrics[currentIndex]);

	const goToPrevious = () => {
		currentIndex = currentIndex > 0 ? currentIndex - 1 : metrics.length - 1;
	};

	const goToNext = () => {
		currentIndex = currentIndex < metrics.length - 1 ? currentIndex + 1 : 0;
	};

	const goToMetric = (index: number) => {
		currentIndex = index;
	};
</script>

{#if currentMetric && metrics.length > 0}
	<div class="flex items-start justify-between pt-2 gap-1 px-1">
		{#if metrics.length > 1}
			<button
				class="btn btn-circle btn-ghost btn-sm self-start"
				onclick={goToPrevious}
				aria-label="Previous metric"
			>
				←
			</button>
		{/if}
		<div class="flex flex-1 flex-row justify-center text-center h-full">
			{#if currentMetric.name}
				{currentMetric.name}
			{:else}
				{currentMetric.metric.toLowerCase()}
			{/if}
		</div>

		<div class="self-start flex flex-row items-center gap-1">
			<div>
				<TrainingMetricMenu
					metric={currentMetric}
					onUpdate={onMetricUpdate}
					onDelete={onMetricUpdate}
				/>
			</div>
			{#if metrics.length > 1}
				<button class="btn btn-circle btn-ghost btn-sm" onclick={goToNext} aria-label="Next metric">
					→
				</button>
			{/if}
		</div>
	</div>

	{#if Object.entries(currentMetric.values).length > 0}
		<div bind:clientWidth={chartWidth}>
			{#if currentMetric.granularity !== null}
				<TrainingMetricsChartStacked
					{height}
					width={chartWidth}
					values={currentMetric.values}
					unit={currentMetric.unit}
					granularity={currentMetric.granularity}
					format={metricValuesDisplayFormat(currentMetric)}
					showGroup={currentMetric.group_by !== null}
					groupBy={currentMetric.group_by}
					stacked={currentMetric.aggregate === 'Sum' ||
						currentMetric.aggregate === 'NumberOfActivities'}
					average={'average' in currentMetric.summary
						? some(currentMetric.summary.average)
						: none()}
				/>
			{:else}
				<TrainingMetricsChartLine
					height={300}
					width={chartWidth}
					values={currentMetric.values}
					unit={currentMetric.unit}
					format={metricValuesDisplayFormat(currentMetric)}
					average={'average' in currentMetric.summary
						? some(currentMetric.summary.average)
						: none()}
					{timeDomain}
				/>
			{/if}
		</div>
	{:else}
		<p class="pb-2 text-center text-sm italic opacity-70">No values found</p>
	{/if}

	{#if metrics.length > 1}
		<div class="flex items-center justify-center gap-2 py-2">
			{#each metrics as _, index}
				<button
					class="h-2 w-2 rounded-full {index === currentIndex ? 'w-6 bg-primary' : 'bg-base-300'}"
					onclick={() => goToMetric(index)}
					aria-label={`Go to metric ${index + 1}`}
				></button>
			{/each}
		</div>
	{/if}
{:else}
	<div class="p-3 text-center text-sm italic opacity-90">No training metrics</div>
{/if}
