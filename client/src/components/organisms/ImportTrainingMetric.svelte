<script lang="ts">
	import CopyUploadButton from '$components/atoms/CopyUploadButton.svelte';
	import { copyTrainingMetricIntoPeriod, type MetricsListGrouped } from '$lib/api';
	import {
		aggregateFunctionDisplay,
		groupByClauseDisplay,
		metricValuesDisplayFormat
	} from '$lib/metric';
	import { isSome, none, some, type Option } from '$lib/Options';
	import TrainingMetricsChartLine from './TrainingMetricsChartLine.svelte';
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';

	let {
		metrics,
		period_id,
		metricCopiedCallback
	}: { metrics: MetricsListGrouped; period_id: string; metricCopiedCallback: () => void } =
		$props();

	let chartWidth: number = $state(300);

	type FormatedMetric = (typeof formatedMetrics)[number];

	let formatedMetrics = $derived(
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

	let tooltipLines = (metric: FormatedMetric) => {
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
		if (metric.groupBy) {
			lines.push({ label: 'Grouped by', value: groupByClauseDisplay(metric.groupBy) });
		}

		// Sports filter
		if (metric.sports && metric.sports.length > 0) {
			lines.push({
				label: 'Filters',
				value: metric.sports.map((s) => s.toLocaleLowerCase()).join(', ')
			});
		} else {
			lines.push({ label: 'Filters', value: 'all sports' });
		}

		return lines;
	};

	let selectedMetric: Option<FormatedMetric> = $state(none());
</script>

<h2 class="fieldset-legend mb-2 text-base">Copy a global metric to this period</h2>
<div class="grid w-full grid-cols-1 gap-4 sm:grid-cols-2 sm:grid-rows-[360px]">
	<div class="flex w-full flex-col gap-3 overflow-scroll">
		{#each formatedMetrics as metric (metric.id)}
			<div
				class="flex w-full flex-row items-center justify-between gap-3 rounded-box bg-base-200 p-3"
			>
				<div class="w-full">
					{metric.name}
				</div>
				<div class="shrink-0">
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
								{#each tooltipLines(metric) as line}
									<div>
										<span class="font-semibold">{line.label}:</span>
										<span class="text-base-content/80">{line.value}</span>
									</div>
								{/each}
							</div>
						</div>
					</div>
					<div class="join">
						<button
							title="View"
							class="btn join-item btn-ghost btn-xs"
							onclick={() => (selectedMetric = some(metric))}
						>
							<img src="/icons/loop.svg" alt="Magnifying glass icon" class="inline h-5 w-5" />
						</button>
						<CopyUploadButton
							onClickCallback={() =>
								copyTrainingMetricIntoPeriod(fetch, metric.id, period_id, metric.name)}
							onSuccessCallback={metricCopiedCallback}
						/>
					</div>
				</div>
			</div>
		{/each}
	</div>

	{#if isSome(selectedMetric)}
		{@const metric = selectedMetric.value}
		{#if metric.values.length > 0}
			<div class="flex w-full flex-col gap-0" bind:clientWidth={chartWidth}>
				{#if metric.granularity !== null}
					<TrainingMetricsChartStacked
						height={300}
						width={chartWidth}
						values={metric.values}
						unit={metric.unit}
						granularity={metric.granularity}
						format={metricValuesDisplayFormat(metric)}
						showGroup={metric.showGroup}
						groupBy={metric.groupBy}
						stacked={metric.aggregate === 'Sum' || metric.aggregate === 'NumberOfActivities'}
						average={'average' in metric.summary ? some(metric.summary.average) : none()}
					/>
				{:else}
					<TrainingMetricsChartLine
						height={300}
						width={chartWidth}
						values={metric.values}
						unit={metric.unit}
						format={metricValuesDisplayFormat(metric)}
						average={'average' in metric.summary ? some(metric.summary.average) : none()}
					/>
				{/if}
			</div>
		{:else}
			<div class="alert rounded-box alert-info">
				<span>No data available for the selected period and filters.</span>
			</div>
		{/if}
	{/if}
</div>
