<script lang="ts">
	import type { TrainingMetricPreviewValues } from '$lib/api';
	import { isNone, isSome, none, some, type Option } from '$lib/Options';
	import type { TrainingMetricFields } from '.';
	import TrainingMetricsChartLine from '../TrainingMetricsChartLine.svelte';
	import TrainingMetricsChartStacked from '../TrainingMetricsChartStacked.svelte';

	let {
		fields,
		values,
		width,
		timeDomain = none()
	}: {
		fields: TrainingMetricFields;
		values: TrainingMetricPreviewValues;
		width: number;
		timeDomain?: Option<{ start: string; end: string | null }>;
	} = $props();

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};
</script>

{#if isSome(fields.granularity)}
	<TrainingMetricsChartStacked
		height={300}
		{width}
		values={values.values}
		unit={values.unit}
		granularity={fields.granularity.value}
		format={previewFormat(values.unit)}
		showGroup={isSome(fields.groupBy)}
		groupBy={isSome(fields.groupBy) ? fields.groupBy.value : null}
		stacked={isNone(fields.selectedTemplate)
			? false
			: fields.selectedTemplate.value.aggregate === 'Sum'
				? true
				: false}
		average={'average' in values.summary ? some(values.summary.average) : none()}
	/>
{:else}
	<TrainingMetricsChartLine
		height={300}
		{width}
		values={values.values}
		unit={values.unit}
		format={previewFormat(values.unit)}
		average={'average' in values.summary ? some(values.summary.average) : none()}
		{timeDomain}
	/>
{/if}
