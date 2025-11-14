<script lang="ts">
	import {
		aggregateFunctionDisplay,
		groupByClauseDisplay,
		type GroupByClause,
		type MetricAggregateFunction
	} from '$lib/metric';

	interface TrainingMetricTitleProps {
		granularity: string;
		aggregate: MetricAggregateFunction;
		metric: string;
		sports?: string[];
		groupBy: GroupByClause | null;
	}

	let { granularity, aggregate, metric, sports, groupBy }: TrainingMetricTitleProps = $props();

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	let subtitle = $derived.by(() => {
		const parts = [];

		// Add sports filter if present
		if (sports && sports.length > 0) {
			parts.push(sports.join(', '));
		} else {
			parts.push('All sports');
		}

		// Add group by if present
		if (groupBy) {
			parts.push(`grouped by ${groupByClauseDisplay(groupBy)}`);
		}

		return parts.join(' · ');
	});
</script>

<div class="flex items-center justify-center gap-1.5">
	<div class="text-base font-medium">
		{capitalize(granularity.toLowerCase())}
		{aggregateFunctionDisplay[aggregate]}
		{#if aggregate !== 'NumberOfActivities'}
			{metric.toLowerCase()}
		{/if}
	</div>
	{#if subtitle}
		<div class="tooltip tooltip-bottom" data-tip={subtitle}>
			<span class="cursor-help text-xs opacity-50">ℹ️</span>
		</div>
	{/if}
</div>
