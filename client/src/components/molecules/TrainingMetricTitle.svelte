<script lang="ts">
	import {
		aggregateFunctionDisplay,
		groupByClauseDisplay,
		type GroupByClause,
		type MetricAggregateFunction
	} from '$lib/metric';

	interface TrainingMetricTitleProps {
		name?: string | null;
		granularity: string;
		aggregate: MetricAggregateFunction;
		metric: string;
		sports?: string[];
		groupBy: GroupByClause | null;
	}

	let { name, granularity, aggregate, metric, sports, groupBy }: TrainingMetricTitleProps =
		$props();

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	let tooltipLines = $derived.by(() => {
		const lines = [];

		// Source metric
		lines.push({ label: 'Metric', value: metric });

		// Granularity
		lines.push({ label: 'Granularity', value: capitalize(granularity.toLowerCase()) });

		// Aggregate function
		lines.push({ label: 'Aggregate', value: aggregateFunctionDisplay[aggregate] });

		// Group by if present
		if (groupBy) {
			lines.push({ label: 'Grouped by', value: groupByClauseDisplay(groupBy) });
		}

		// Sports filter
		if (sports && sports.length > 0) {
			lines.push({ label: 'Sports', value: sports.join(', ') });
		} else {
			lines.push({ label: 'Sports', value: 'All sports' });
		}

		return lines;
	});
</script>

<div class="flex flex-row items-center justify-center gap-1.5">
	<div class="font-medium">
		{#if name}
			{name}
		{:else}
			{capitalize(granularity.toLowerCase())}
			{aggregateFunctionDisplay[aggregate]}
			{#if aggregate !== 'NumberOfActivities'}
				{metric.toLowerCase()}
			{/if}
		{/if}
	</div>
	<div class="dropdown-hover dropdown dropdown-end dropdown-bottom">
		<div tabindex="0" role="button" class="cursor-help text-xs opacity-50">
			<img src="/icons/info.svg" class="h-6 w-6" alt="Information icon" />
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
</div>
