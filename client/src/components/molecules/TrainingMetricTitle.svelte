<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { metricScope, type MetricsListItemGrouped } from '$lib/api';
	import { aggregateFunctionDisplay, groupByClauseDisplay } from '$lib/metric';
	import TrainingMetricMenu from './TrainingMetricMenu.svelte';

	let { metric }: { metric: MetricsListItemGrouped } = $props();

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	let tooltipLines = $derived.by(() => {
		const lines = [];

		// Source metric
		lines.push({ label: 'Metric', value: metric.metric });

		// Granularity
		lines.push({ label: 'Granularity', value: capitalize(metric.granularity.toLowerCase()) });

		// Aggregate function
		lines.push({ label: 'Aggregate', value: aggregateFunctionDisplay[metric.aggregate] });

		// Group by if present
		if (metric.group_by) {
			lines.push({ label: 'Grouped by', value: groupByClauseDisplay(metric.group_by) });
		}

		// Sports filter
		if (metric.sports && metric.sports.length > 0) {
			lines.push({ label: 'Sports', value: metric.sports.join(', ') });
		} else {
			lines.push({ label: 'Sports', value: 'All sports' });
		}

		return lines;
	});
</script>

<div class="relative w-full p-4 text-center">
	<div class="flex flex-row items-center justify-center gap-1.5">
		<div class=" font-medium">
			{#if metric.name}
				{metric.name}
			{:else}
				{capitalize(metric.granularity.toLowerCase())}
				{aggregateFunctionDisplay[metric.aggregate]}
				{#if metric.aggregate !== 'NumberOfActivities'}
					{metric.metric.toLowerCase()}
				{/if}
			{/if}
		</div>

		<div class="absolute right-4 bottom-[16px] flex flex-row items-center gap-0.5">
			<!-- Action menu dropdown -->

			<div class="dropdown-hover dropdown dropdown-end dropdown-bottom">
				<div tabindex="0" role="button" class="cursor-help text-xs opacity-50">
					<img src="/icons/info.svg" class="h-5 w-5" alt="Information icon" />
				</div>
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div
					tabindex="0"
					class="dropdown-content z-10 rounded-box bg-base-200 p-3 shadow-lg"
					style="min-width: 200px;"
				>
					<div class="space-y-1 text-left text-sm">
						{#each tooltipLines as line}
							<div class="flex gap-2">
								<span class="font-semibold">{line.label}:</span>
								<span class="text-base-content/80">{line.value}</span>
							</div>
						{/each}
					</div>
				</div>
			</div>
			<TrainingMetricMenu
				name={metric.name}
				id={metric.id}
				scope={metricScope(metric)}
				onUpdate={() => invalidate('app:training-metrics')}
				onDelete={() => invalidate('app:training-metrics')}
			/>
		</div>
	</div>
</div>
