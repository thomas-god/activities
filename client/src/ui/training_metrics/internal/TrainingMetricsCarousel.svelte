<script lang="ts">
	import type { TrainingMetric } from '$lib/api/training';
	import { none, some, type Option } from '$lib/Options';
	import TrainingMetricMenu from '$ui/training_metrics/internal/TrainingMetricMenu.svelte';
	import TrainingMetricChart from '../TrainingMetricChart.svelte';

	let {
		metrics,
		height,
		onMetricUpdate,
		timeDomain = none()
	}: {
		metrics: TrainingMetric[];
		height: number;
		onMetricUpdate: () => void;
		timeDomain?: Option<{ start: string; end: string | null }>;
	} = $props();

	let chartWidth: number = $state(300);
	let currentIndex = $derived(0);
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
	<div class="flex items-start justify-between gap-1 px-1 pt-2">
		{#if metrics.length > 1}
			<button
				class="btn btn-circle self-start btn-ghost btn-sm"
				onclick={goToPrevious}
				aria-label="Previous metric"
			>
				←
			</button>
		{/if}
		<div class="flex h-full flex-1 flex-row justify-center text-center">
			{#if currentMetric.name}
				{currentMetric.name}
			{:else}
				{currentMetric.metric.toLowerCase()}
			{/if}
		</div>

		<div class="flex flex-row items-center gap-1 self-start">
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

	<div bind:clientWidth={chartWidth}>
		<TrainingMetricChart metric={currentMetric} width={chartWidth} {height} {timeDomain} />
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
