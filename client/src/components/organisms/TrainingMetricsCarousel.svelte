<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import type { MetricsListItemGrouped } from '$lib/api/training';
	import { metricValuesDisplayFormat } from '$lib/metric';

	let {
		metrics,
		height,
		initialIndex = 0
	}: {
		metrics: MetricsListItemGrouped[];
		height: number;
		initialIndex?: number;
	} = $props();

	let chartWidth: number = $state(300);
	let currentIndex = $state(initialIndex);

	let currentMetric = $derived.by(() => {
		const metric = metrics[currentIndex];
		if (!metric) return undefined;

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
			initialMetric: metric
		};
	});

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
	<div class="flex items-center justify-between">
		<button
			class="btn btn-circle btn-ghost btn-sm"
			onclick={goToPrevious}
			aria-label="Previous metric"
		>
			←
		</button>
		<div class="flex flex-1 flex-row justify-center text-center">
			<TrainingMetricTitle metric={currentMetric.initialMetric} />
		</div>
		<button class="btn btn-circle btn-ghost btn-sm" onclick={goToNext} aria-label="Next metric">
			→
		</button>
	</div>

	<div bind:clientWidth={chartWidth}>
		<TrainingMetricsChartStacked
			{height}
			width={chartWidth}
			values={currentMetric.values}
			unit={currentMetric.unit}
			granularity={currentMetric.granularity}
			format={metricValuesDisplayFormat(currentMetric)}
			showGroup={currentMetric.showGroup}
			groupBy={currentMetric.groupBy}
		/>
	</div>

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
