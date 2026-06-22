<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import { isNone, none, some, type Option } from '$lib/Options';
	import TrainingMetricForm from '$components/organisms/training_metric/TrainingMetricForm.svelte';
	import {
		fieldsAsPayload,
		type Scope,
		type TrainingMetricFields
	} from '$components/organisms/training_metric';
	import {
		createTrainingMetric,
		fetchTrainingMetricTemplates,
		getTrainingMetricPreview,
		type CreateTrainingMetricPayload,
		type PreviewTrainingMetricPayload
	} from '$lib/api';
	import PreviewChart from '$components/organisms/training_metric/PreviewChart.svelte';

	let {
		callback,
		scope = { kind: 'global' },
		existingSportsConstraints = none()
	}: {
		callback: () => void;
		scope?: Scope;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	const buildPromise = async () => {
		const templates = await fetchTrainingMetricTemplates();
		if (templates.length > 0) {
			fields = { ...fields, selectedTemplate: some(templates[0]) };
		}
		return templates;
	};
	let metricTemplatesPromise = $state(buildPromise());

	let fields: TrainingMetricFields = $state({
		name: '',
		selectedTemplate: none(),
		granularity: none(),
		groupBy: none(),
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

	let requestPending = $state(false);

	let metricDefinitionPayload = $derived(fieldsAsPayload(fields));

	let previewRequest: Option<PreviewTrainingMetricPayload> = $derived.by(() => {
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

	let createMetricRequest: Option<CreateTrainingMetricPayload> = $derived.by(() => {
		if (isNone(metricDefinitionPayload)) {
			return none();
		}
		if (fields.name.trim() === '') {
			return none();
		}

		return some({
			...metricDefinitionPayload.value,
			name: fields.name.trim(),
			scope:
				scope.kind === 'global'
					? { type: 'global' }
					: { type: 'trainingPeriod', trainingPeriodId: scope.periodId }
		});
	});

	const createMetricCallback = async (
		payload: Option<CreateTrainingMetricPayload>
	): Promise<void> => {
		if (isNone(payload)) {
			return;
		}
		await createTrainingMetric(payload.value);
		callback();
	};

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
						<PreviewChart width={chartWidth} {values} {fields} />
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
