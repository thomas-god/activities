<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import TrainingMetricsChartStacked from '../organisms/TrainingMetricsChartStacked.svelte';
	import { isNone, none, some, type Option } from '$lib/Options';
	import TrainingMetricsChartLine from '$components/organisms/TrainingMetricsChartLine.svelte';
	import TrainingMetricForm from '$components/organisms/training_metric/TrainingMetricForm.svelte';
	import {
		fieldsAsPayload,
		type Scope,
		type TrainingMetricFields
	} from '$components/organisms/training_metric';
	import {
		createTrainingMetric,
		fetchTrainingMetricTemplates,
		getTrainingMetricPreview
	} from '$lib/api';

	let {
		callback,
		scope = { kind: 'global' },
		existingSportsConstraints = none()
	}: {
		callback: () => void;
		scope?: Scope;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	let metricTemplatesPromise = $state(fetchTrainingMetricTemplates());

	let fields: TrainingMetricFields = $state({
		name: '',
		selectedTemplate: none(),
		granularity: 'None',
		groupBy: 'None',
		filters: {
			sports: none(),
			sportCategories: none(),
			rpes: none(),
			bonked: none(),
			workoutTypes: none()
		},
		showAverage: false
	});

	let chartWidth: number = $state(0);

	const previewFormat = (unit: string): 'number' | 'duration' | 'pace' => {
		if (unit === 'activities') return 'number';
		if (unit === 's') return 'duration';
		if (unit === 's/km') return 'pace';
		return 'number';
	};

	let requestPending = $state(false);

	let metricDefinitionPayload = $derived(fieldsAsPayload(fields));

	let previewRequest: Option<Object> = $derived.by(() => {
		if (isNone(metricDefinitionPayload)) {
			return none();
		}
		const start = dayjs().subtract(3, 'weeks').format('YYYY-MM-DDTHH:mm:ssZ');
		const end = dayjs().format('YYYY-MM-DDTHH:mm:ssZ');

		return some({ start, end, ...metricDefinitionPayload.value });
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
		if (isNone(metricDefinitionPayload)) {
			return none();
		}
		if (fields.name.trim() === '') {
			return none();
		}

		return some({ name: fields.name.trim(), ...metricDefinitionPayload.value });
	});

	const createMetricCallback = async (payload: Option<Object>): Promise<void> => {
		if (isNone(payload)) {
			return;
		}
		const _scope =
			scope.kind === 'global'
				? { type: 'global' }
				: { type: 'trainingPeriod', trainingPeriodId: scope.periodId };
		await createTrainingMetric({
			scope: _scope,
			...payload.value
		});
		callback();
	};

	// TODO: do not run this at component init ?
	const fetchPreview = async (request: typeof previewRequest) => {
		if (isNone(request)) {
			return { values: [], unit: '', summary: {} };
		}

		const values = await getTrainingMetricPreview(request.value);

		if (isNone(values)) {
			return { values: [], unit: '', summary: {} };
		}

		return values.value;
	};
</script>

<div class="grid grid-cols-1 gap-4 text-sm sm:grid-cols-2" id="test-debug">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<legend class="fieldset-legend text-base">New training metric</legend>

		{#await metricTemplatesPromise then metricTemplates}
			<TrainingMetricForm templates={metricTemplates} bind:fields {existingSportsConstraints} />

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
		{/await}
	</fieldset>

	<div class="hidden self-center sm:block">
		{#key previewKey}
			{#await previewData}
				<div class="p-8 text-center">
					<span class="loading loading-lg loading-spinner"></span>
				</div>
			{:then values}
				{#if values.values.length > 0}
					<div bind:clientWidth={chartWidth}>
						{#if fields.granularity !== 'None'}
							<TrainingMetricsChartStacked
								height={300}
								width={chartWidth}
								values={values.values}
								unit={values.unit}
								granularity={fields.granularity}
								format={previewFormat(values.unit)}
								showGroup={fields.groupBy !== 'None'}
								groupBy={fields.groupBy !== 'None' ? fields.groupBy : null}
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
								width={chartWidth}
								values={values.values}
								unit={values.unit}
								format={previewFormat(values.unit)}
								average={'average' in values.summary ? some(values.summary.average) : none()}
							/>
						{/if}
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
