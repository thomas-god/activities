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

	let metricProps = $derived(
		metrics.map((metric) => {
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
				initialMetric: metric,
				summary: metric.summary
			};
		})
	);
</script>

<div class="flex flex-col items-center gap-0">
	{#each metrics as metric, idx (metric.id)}
		<div class="flex w-full flex-col gap-0" bind:clientWidth={chartWidth}>
			<div class="px-4 pt-4">
				<TrainingMetricTitle {metric} onUpdate={onMetricUpdate} />
			</div>
			<TrainingMetricChart {metric} width={chartWidth} {height} {timeDomain} />

			{#if idx !== metricProps.length - 1}
				<div class="divider"></div>
			{/if}
		</div>
	{:else}
		<div class="p-3 text-center text-sm tracking-wide italic opacity-60">No training metrics</div>
	{/each}
</div>
