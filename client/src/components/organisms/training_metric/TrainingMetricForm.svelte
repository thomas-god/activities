<script lang="ts">
	import { type Sport, type SportCategory } from '$lib/sport';
	import { isNone, none, some, type Option } from '$lib/Options';
	import type { MetricTemplate } from '$lib/api';
	import TrainingMetricFilters from '../TrainingMetricFilters.svelte';
	import type { TrainingMetricFields } from '.';

	let {
		templates,
		fields = $bindable(),
		existingSportsConstraints = none()
	}: {
		templates: MetricTemplate[];
		fields: TrainingMetricFields;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	let templatesByCategory = $derived.by(() => {
		const groupedMetrics: Map<string, MetricTemplate[]> = new Map();
		for (const template of templates) {
			groupedMetrics.getOrInsert(template.category, []).push(template);
		}
		return groupedMetrics;
	});
</script>

<label class="label" for="metric-source"> Metric to extract from each activity </label>
<select
	id="metric-source"
	class="select w-full"
	bind:value={
		() => {
			if (isNone(fields.selectedTemplate)) {
				return null;
			}
			return fields.selectedTemplate.value;
		},
		(v) => {
			if (v === null) {
				fields = { ...fields, selectedTemplate: none() };
			} else {
				fields = { ...fields, selectedTemplate: some(v) };
			}
		}
	}
>
	{#each templatesByCategory as [category, group]}
		<optgroup label={category}>
			{#each group as template}
				<option value={template}>{template.display_name}</option>
				{template}
			{/each}
		</optgroup>
	{/each}
</select>

<label class="label" for="metric-granularity">Group activities by</label>
<select
	class="select w-full"
	bind:value={() => fields.granularity, (g) => (fields = { ...fields, granularity: g })}
	id="metric-granularity"
>
	<option value="None">None</option>
	<option value="Daily">Day</option>
	<option value="Weekly">Week</option>
	<option value="Monthly">Month</option>
</select>

{#if fields.granularity !== 'None'}
	<label class="label" for="metric-group-by">Additionally group activities by</label>
	<select
		class="select w-full"
		bind:value={() => fields.groupBy, (g) => (fields = { ...fields, groupBy: g })}
		id="metric-group-by"
	>
		<option value="None">No grouping</option>
		<option value="Sport">Sport</option>
		<option value="SportCategory">Sport Category</option>
		<option value="WorkoutType">Workout Type</option>
		<option value="RpeRange">RPE Range</option>
		<option value="Bonked">Bonked</option>
	</select>
{/if}

<TrainingMetricFilters
	bind:filters={() => fields.filters, (f) => (fields = { ...fields, filters: f })}
	{existingSportsConstraints}
/>

<label class="label" for="show-metric-average">
	Display metric average
	<input
		type="checkbox"
		class="checkbox checkbox-sm"
		id="show-metric-average"
		bind:checked={() => fields.showAverage, (c) => (fields = { ...fields, showAverage: c })}
	/>
</label>

<label class="label" for="metric-name">Metric name</label>
<input
	type="text"
	class="input"
	id="metric-name"
	bind:value={() => fields.name, (n) => (fields = { ...fields, name: n })}
	placeholder="e.g., Weekly running volume"
	required
/>
