<script lang="ts">
	import { type TrainingMetric } from '$lib/api';
	import { aggregateFunctionDisplay } from '$lib/trainingMetric';
	import TrainingMetricMenu from './TrainingMetricMenu.svelte';

	let { metric, onUpdate }: { metric: TrainingMetric; onUpdate: () => void } = $props();

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');
</script>

<div class="w-full flex flex-row items-center justify-center gap-1.5">
	<div class="w-full font-medium text-center">
		{#if metric.name}
			{metric.name}
		{:else}
			{#if metric.granularity !== null}
				{capitalize(metric.granularity.toLowerCase())}
			{/if}
			{#if metric.aggregate !== null}
				{aggregateFunctionDisplay[metric.aggregate]}
			{/if}
			{#if metric.aggregate !== 'NumberOfActivities'}
				{metric.metric.toLowerCase()}
			{/if}
		{/if}
	</div>

	<div class="self-start">
		<TrainingMetricMenu {metric} {onUpdate} onDelete={onUpdate} />
	</div>
</div>
