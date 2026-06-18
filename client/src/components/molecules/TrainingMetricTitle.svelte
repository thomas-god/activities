<script lang="ts">
	import { metricScope, type TrainingMetric } from '$lib/api';
	import { aggregateFunctionDisplay, groupByClauseDisplay } from '$lib/trainingMetric';
	import TrainingMetricMenu from './TrainingMetricMenu.svelte';

	let { metric, onUpdate }: { metric: TrainingMetric; onUpdate: () => void } = $props();

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	let tooltipLines = $derived.by(() => {
		const lines = [];

		// Source metric
		lines.push({ label: 'Source', value: metric.metric.toLocaleLowerCase() });

		// Granularity
		if (metric.granularity !== null) {
			lines.push({ label: 'Granularity', value: metric.granularity.toLowerCase() });
		}

		// Aggregate function
		if (metric.aggregate !== null) {
			lines.push({ label: 'Aggregate', value: aggregateFunctionDisplay[metric.aggregate] });
		}

		// Group by if present
		if (metric.group_by) {
			lines.push({ label: 'Grouped by', value: groupByClauseDisplay(metric.group_by) });
		}

		// Sports filter
		if (metric.sports && metric.sports.sports.length + metric.sports.categories.length > 0) {
			const sports = (metric.sports.sports as string[]).concat(metric.sports.categories);
			lines.push({
				label: 'Filters',
				value: sports.join(', ')
			});
		} else {
			lines.push({ label: 'Filters', value: 'all sports' });
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
							<div>
								<span class="font-semibold">{line.label}:</span>
								<span class="text-base-content/80">{line.value}</span>
							</div>
						{/each}
					</div>
				</div>
			</div>
			<TrainingMetricMenu
				{metric}
				name={metric.name}
				id={metric.id}
				scope={metricScope(metric)}
				{onUpdate}
				onDelete={onUpdate}
			/>
		</div>
	</div>
</div>
