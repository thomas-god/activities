<script lang="ts">
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import type { MetricsListItemGrouped } from '$lib/api/training';

	let {
		metrics,
		height,
		initialIndex = 0,
		favoriteMetricId
	}: {
		metrics: MetricsListItemGrouped[];
		height: number;
		initialIndex?: number;
		favoriteMetricId?: string | null;
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
		<div class="flex-1 text-center">
			<TrainingMetricTitle
				name={currentMetric.name}
				granularity={currentMetric.granularity}
				aggregate={currentMetric.aggregate}
				metric={currentMetric.metric}
				sports={currentMetric.sports}
				groupBy={currentMetric.groupBy}
				isFavorite={currentMetric.id === favoriteMetricId}
			/>
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
			format={currentMetric.unit === 's' ? 'duration' : 'number'}
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
{/if}
