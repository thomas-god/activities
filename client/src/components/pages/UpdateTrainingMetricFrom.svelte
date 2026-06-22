<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import { isNone, none, some, type Option } from '$lib/Options';
	import TrainingMetricForm from '$components/organisms/training_metric/TrainingMetricForm.svelte';
	import {
		fieldsAsPayload,
		matchMetricToFormFields,
		type TrainingMetricFields
	} from '$components/organisms/training_metric';
	import {
		fetchTrainingMetricTemplates,
		getTrainingMetricPreview,
		updateTrainingMetric,
		type PreviewTrainingMetricPayload,
		type TrainingMetric,
		type UpdateTrainingMetricPayload
	} from '$lib/api';
	import PreviewChart from '$components/organisms/training_metric/PreviewChart.svelte';

	let {
		initialMetric,
		callback,
		existingSportsConstraints = none()
	}: {
		initialMetric: TrainingMetric;
		callback: () => void;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	const buildPromise = async () => {
		const templates = await fetchTrainingMetricTemplates();
		fields = matchMetricToFormFields(initialMetric, templates);
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

	let dates = {
		start: dayjs().subtract(4, 'weeks').format('YYYY-MM-DDTHH:mm:ssZ'),
		end: dayjs().add(1, 'day').format('YYYY-MM-DDTHH:mm:ssZ')
	};
	let previewRequest: Option<PreviewTrainingMetricPayload> = $derived.by(() => {
		if (isNone(metricDefinitionPayload)) {
			return none();
		}

		return some({ ...dates, ...metricDefinitionPayload.value });
	});

	// Automatically fetch preview when form values change
	let previewData = $derived.by(() => {
		// Access previewRequest to track its dependencies
		const request = previewRequest;
		return fetchPreview(request);
	});

	// Create a unique key for the preview request to force re-rendering
	let previewKey = $derived(JSON.stringify(previewRequest));

	let updateMetricRequest: Option<UpdateTrainingMetricPayload> = $derived.by(() => {
		if (isNone(metricDefinitionPayload)) {
			return none();
		}
		if (fields.name.trim() === '') {
			return none();
		}

		return some({ name: fields.name.trim(), ...metricDefinitionPayload.value });
	});

	const updateMetricCallback = async (
		payload: Option<UpdateTrainingMetricPayload>
	): Promise<void> => {
		if (isNone(payload)) {
			return;
		}

		await updateTrainingMetric(initialMetric.id, payload.value);
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

<div class="grid grid-cols-1 gap-4 text-start text-sm sm:grid-cols-2" id="test-debug">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<legend class="fieldset-legend text-base">Update training metric</legend>

		{#await metricTemplatesPromise then metricTemplates}
			<TrainingMetricForm templates={metricTemplates} bind:fields {existingSportsConstraints} />

			<div class="mt-4">
				<button
					class="btn w-full btn-neutral"
					onclick={async () => {
						requestPending = true;
						await updateMetricCallback(updateMetricRequest);
						requestPending = false;
					}}
					disabled={requestPending || isNone(updateMetricRequest)}
					>Update metric
					{#if requestPending}
						<span class="loading loading-spinner"></span>
					{/if}
				</button>
			</div>
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
						<PreviewChart width={chartWidth} {values} {fields} timeDomain={some(dates)} />
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
