<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import { workoutTypeToAPI } from '$lib/workout-type';
	import { bonkStatusToAPI } from '$lib/nutrition';
	import TrainingMetricsChartStacked from '../organisms/TrainingMetricsChartStacked.svelte';
	import z from 'zod';
	import { isNone, isSome, map, none, some, unwrapOr, type Option } from '$lib/Options';
	import TrainingMetricFilters, {
		type TrainingMetricFiltersType
	} from '../organisms/TrainingMetricFilters.svelte';

	export type Scope = { kind: 'global' } | { kind: 'period'; periodId: string };

	let {
		callback,
		scope = { kind: 'global' },
		existingSportsConstraints = none()
	}: {
		callback: () => void;
		scope?: Scope;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	const MetricTemplatesSchema = z.array(
		z.object({
			display_name: z.string(),
			metric: z.string(),
			aggregate: z.string(),
			unit: z.string(),
			category: z.string()
		})
	);
	type MetricTemplate = z.infer<typeof MetricTemplatesSchema>[number];

	let buildMetricTemplatesPromise = async () => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metrics/templates`, {
			method: 'GET',
			mode: 'cors',
			credentials: 'include'
		});

		return MetricTemplatesSchema.parse(await res.json());
	};
	let metricTemplatesPromise = $state(buildMetricTemplatesPromise());

	const groupTemplatesByCategory = (templates: MetricTemplate[]): Map<string, MetricTemplate[]> => {
		const groupedMetrics: Map<string, MetricTemplate[]> = new Map();
		for (const template of templates) {
			groupedMetrics.getOrInsert(template.category, []).push(template);
		}
		return groupedMetrics;
	};

	let selectedTemplate: Option<MetricTemplate> = $state(none());
	const granularityValues = ['Daily', 'Weekly', 'Monthly'] as const;
	type Granularity = (typeof granularityValues)[number];

	let granularity: Granularity = $state('Weekly');
	let groupBy: 'None' | 'Sport' | 'SportCategory' | 'WorkoutType' | 'RpeRange' | 'Bonked' =
		$state('None');
	let filters: TrainingMetricFiltersType = $state({
		sports: none(),
		sportCategories: none(),
		rpes: none(),
		bonked: none(),
		workoutTypes: none()
	});
	let metricName = $state('');

	let chartWidth: number = $state(0);

	let previewUnit = $derived(
		unwrapOr(
			map(selectedTemplate, (template) => template.unit),
			''
		)
	);

	let previewFormat: 'number' | 'duration' | 'pace' = $derived.by(() => {
		// if (aggregate === 'NumberOfActivities') return 'number';
		if (previewUnit === 's') return 'duration';
		if (previewUnit === 's/km') return 'pace';
		return 'number';
	});

	let requestPending = $state(false);

	// Build filters object based on selected filter values
	let activeFilters = $derived.by(() => {
		let activeFilters: any = {};

		if (isSome(filters.sports) && isSome(filters.sportCategories)) {
			const sportFilter = filters.sports.value.map((sport) => ({
				Sport: sport
			}));
			const sportCategoriesFilter = filters.sportCategories.value.map((category) => ({
				SportCategory: category
			}));
			const sportFilters: ({ Sport: Sport } | { SportCategory: SportCategory })[] = [
				...sportFilter,
				...sportCategoriesFilter
			];
			if (sportFilters.length > 0) {
				activeFilters.sports = sportFilters;
			}
		}

		if (isSome(filters.workoutTypes) && filters.workoutTypes.value.length > 0) {
			activeFilters.workout_types = filters.workoutTypes.value.map(workoutTypeToAPI);
		}

		if (isSome(filters.bonked)) {
			activeFilters.bonked = bonkStatusToAPI(filters.bonked.value);
		}

		if (isSome(filters.rpes) && filters.rpes.value.length > 0) {
			activeFilters.rpes = filters.rpes.value;
		}

		return activeFilters;
	});

	let previewRequest = $derived.by(() => {
		if (isNone(selectedTemplate)) {
			return none();
		}
		const start = dayjs().subtract(3, 'weeks').format('YYYY-MM-DDTHH:mm:ssZ');
		const end = dayjs().format('YYYY-MM-DDTHH:mm:ssZ');

		let payload: {
			metric: string;
			window: {
				granularity: string;
				aggregate: string;
				group_by?: Exclude<typeof groupBy, 'None'>;
			};
			filters?: {};
			start: string;
			end: string;
		} = {
			metric: selectedTemplate.value.metric,
			window: {
				granularity: granularity,
				aggregate: selectedTemplate.value.aggregate
			},
			start,
			end
		};

		if (Object.keys(activeFilters).length > 0) {
			payload = { ...payload, filters: activeFilters };
		}

		if (groupBy !== 'None') {
			payload.window = { ...payload.window, group_by: groupBy };
		}

		return some(payload);
	});

	// Automatically fetch preview when form values change
	let previewData = $derived.by(() => {
		// Access previewRequest to track its dependencies
		const request = previewRequest;
		return fetchPreview(request);
	});

	// Create a unique key for the preview request to force re-rendering
	let previewKey = $derived(JSON.stringify(previewRequest));

	let createMetricRequest: Option<Object> = $derived.by(() => {
		if (isNone(selectedTemplate)) {
			return none();
		}
		if (metricName.trim() === '') {
			return none();
		}
		let basePayload: {
			name: string;
			metric: string;
			window: {
				granularity: string;
				aggregate: string;
				group_by?: Exclude<typeof groupBy, 'None'>;
			};
			filters: {};
		} = {
			name: metricName.trim(),
			metric: selectedTemplate.value.metric,
			window: {
				granularity,
				aggregate: selectedTemplate.value.aggregate
			},
			filters: activeFilters
		};

		if (groupBy !== 'None') {
			basePayload.window = { ...basePayload.window, group_by: groupBy };
		}

		return some(basePayload);
	});

	const createMetricCallback = async (payload: Option<Object>): Promise<void> => {
		if (isNone(payload)) {
			return;
		}
		const _scope =
			scope.kind === 'global'
				? { type: 'global' }
				: { type: 'trainingPeriod', trainingPeriodId: scope.periodId };
		const body = JSON.stringify({
			scope: _scope,
			...payload.value
		});
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric`, {
			body,
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto('/login');
		}
		invalidate('app:training-metrics');
		callback();
	};

	// TODO: do not run this at component init ?
	const fetchPreview = async (
		request: typeof previewRequest
	): Promise<{ time: string; group: string; value: number }[]> => {
		if (isNone(request)) {
			return [];
		}

		const body = JSON.stringify(request.value);
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/values`, {
			body,
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto('/login');
			throw new Error('Unauthorized');
		}

		if (res.status !== 200) {
			throw new Error('Failed to fetch preview');
		}

		const data = await res.json();
		// TODO: get unit from data.unit
		// Transform the GroupedMetricValues response to the format expected by the chart
		// Response format: { group_name: { granule: value } }
		const values: { time: string; group: string; value: number }[] = [];
		for (const [group, granuleValues] of Object.entries(data.values)) {
			for (const [time, value] of Object.entries(granuleValues as Record<string, number>)) {
				values.push({ time, group, value });
			}
		}
		return values;
	};
</script>

<div class="grid grid-cols-1 gap-4 text-sm sm:grid-cols-2" id="test-debug">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<legend class="fieldset-legend text-base">New training metric</legend>

		{#await metricTemplatesPromise then metricTemplates}
			{@const groupedTemplates = groupTemplatesByCategory(metricTemplates)}
			<label class="label" for="metric-source"> Metric to extract from each activity </label>
			<select
				id="metric-source"
				class="select w-full"
				bind:value={
					() => {
						if (isNone(selectedTemplate)) {
							return null;
						}
						return selectedTemplate.value;
					},
					(v) => {
						if (v === null) {
							selectedTemplate = none();
						} else {
							selectedTemplate = some(v);
						}
					}
				}
			>
				{#each groupedTemplates as [category, group]}
					<optgroup label={category}>
						{#each group as template}
							<option value={template}>{template.display_name}</option>
							{template}
						{/each}
					</optgroup>
				{/each}
			</select>
		{/await}

		<label class="label" for="metric-granularity">Group activities by</label>
		<select class="select w-full" bind:value={granularity} id="metric-granularity">
			<option value="Daily">Day</option>
			<option value="Weekly">Week</option>
			<option value="Monthly">Month</option>
		</select>

		<label class="label" for="metric-group-by">Additionally group activities by</label>
		<select class="select w-full" bind:value={groupBy} id="metric-group-by">
			<option value="None">No grouping</option>
			<option value="Sport">Sport</option>
			<option value="SportCategory">Sport Category</option>
			<option value="WorkoutType">Workout Type</option>
			<option value="RpeRange">RPE Range</option>
			<option value="Bonked">Bonked</option>
		</select>

		<TrainingMetricFilters bind:filters {existingSportsConstraints} />

		<label class="label" for="metric-name">Metric name</label>
		<input
			type="text"
			class="input"
			id="metric-name"
			bind:value={metricName}
			placeholder="e.g., Weekly running volume"
			required
		/>

		<div class="mt-4">
			<button
				class="btn w-full btn-neutral"
				onclick={async () => {
					requestPending = true;
					await createMetricCallback(createMetricRequest);
					requestPending = false;
				}}
				disabled={requestPending || isNone(createMetricRequest)}
				>Create metric
				{#if requestPending}
					<span class="loading loading-spinner"></span>
				{/if}
			</button>
		</div>
		{#if scope.kind === 'period'}
			<div class="p-1 italic opacity-90">
				This metric will only be visible to the current training period
			</div>
		{/if}
	</fieldset>

	<div class="hidden self-center sm:block">
		{#key previewKey}
			{#await previewData}
				<div class="p-8 text-center">
					<span class="loading loading-lg loading-spinner"></span>
				</div>
			{:then values}
				{#if values.length > 0}
					<div bind:clientWidth={chartWidth}>
						<TrainingMetricsChartStacked
							height={300}
							width={chartWidth}
							{values}
							unit={previewUnit}
							{granularity}
							format={previewFormat}
							showGroup={groupBy !== 'None'}
							groupBy={groupBy !== 'None' ? groupBy : null}
							stacked={false}
						/>
					</div>
				{:else}
					<div class="alert rounded-box alert-info">
						<span>No data available for the selected period and filters.</span>
					</div>
				{/if}
			{:catch error}
				<div class="alert rounded-box alert-error">
					<span>Failed to load preview. Please try again.</span>
				</div>
			{/await}
		{/key}
	</div>
</div>
