<script lang="ts">
	import { none, type Option } from '$lib/Options';
	import type { TrainingMetric } from '$lib/api/training';
	import TrainingMetricTitle from '$ui/training_metrics/TrainingMetricTitle.svelte';
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
</script>

<div class="flex flex-col items-center gap-0">
	{#each metrics as metric, idx (metric.id)}
		<div class="flex w-full flex-col gap-0" bind:clientWidth={chartWidth}>
			<div class="px-4 pt-4">
				<TrainingMetricTitle {metric} onUpdate={onMetricUpdate} />
			</div>
			<TrainingMetricChart {metric} width={chartWidth} {height} {timeDomain} />

			{#if idx !== metrics.length - 1}
				<div class="divider"></div>
			{/if}
		</div>
	{:else}
		<div class="p-3 text-center text-sm tracking-wide italic opacity-60">No training metrics</div>
	{/each}
</div>
