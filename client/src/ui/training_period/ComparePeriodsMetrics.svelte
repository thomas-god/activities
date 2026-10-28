<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import {
		fetchTrainingMetricTemplates,
		fetchTrainingPeriodMetrics,
		getTrainingMetricPreview,
		type TrainingMetric,
		type TrainingMetricList,
		type TrainingPeriodDetails
	} from '#lib/api/index.js';
	import { isSome, map, type Option } from '#lib/Options.js';
	import {
		definitionLabel,
		metricPreviewPayload,
		metricDefinitionKey,
		extractBaseDefinitionFromMetric,
		definitionBaseKey,
		defaultCompareDefinitions,
		type CompareAlignment,
		type CompareMetricDefinition
	} from '#lib/trainingMetric.js';
	import CompareMetricEntry from './internal/CompareMetricEntry.svelte';
	import TrainingMetricForm from '#ui/training_metrics/TrainingMetricForm.svelte';
	import {
		emptyTrainingMetricFields,
		fieldsAreEmpty,
		fieldsAsPayload,
		type TrainingMetricFields
	} from '#ui/training_metrics';

	let {
		firstPeriod,
		secondPeriod
	}: { firstPeriod: TrainingPeriodDetails; secondPeriod: TrainingPeriodDetails } = $props();

	let alignBy: CompareAlignment = $state('start');

	/** One compared metric with its already-started computation per period. */
	type MetricComparison = {
		definition: CompareMetricDefinition;
		firstPeriodValues: Promise<Option<TrainingMetric>>;
		secondPeriodValues: Promise<Option<TrainingMetric>>;
	};

	const buildMetricComparison = (definition: CompareMetricDefinition): MetricComparison => ({
		definition,
		firstPeriodValues: getTrainingMetricPreview(metricPreviewPayload(definition, firstPeriod)),
		secondPeriodValues: getTrainingMetricPreview(metricPreviewPayload(definition, secondPeriod))
	});

	/**
	 * Merges the saved metrics of both periods. Metrics sharing the same
	 * definition are compared once and marked as coming from both periods.
	 */
	const mergedCompareDefinitions = (
		firstPeriodMetrics: TrainingMetricList,
		secondPeriodMetrics: TrainingMetricList,
		firstPeriodName: string,
		secondPeriodName: string
	): CompareMetricDefinition[] => {
		const merged: Record<string, CompareMetricDefinition> = {};

		for (const metric of firstPeriodMetrics) {
			const key = metricDefinitionKey(metric);
			merged[key] = {
				key,
				label: metric.name,
				source: firstPeriodName,
				base: extractBaseDefinitionFromMetric(metric)
			};
		}

		for (const metric of secondPeriodMetrics) {
			const key = metricDefinitionKey(metric);
			const existing = merged[key];
			if (existing !== undefined) {
				existing.source = 'both';
			} else {
				merged[key] = {
					key,
					label: metric.name,
					source: secondPeriodName,
					base: extractBaseDefinitionFromMetric(metric)
				};
			}
		}

		return Object.values(merged).toSorted((a, b) =>
			definitionLabel(a).localeCompare(definitionLabel(b))
		);
	};

	// Custom metrics added by the user, appended after the automatic ones, and
	// keys the user removed (built-in and period metrics are recomputed each
	// time, so they must be filtered out on display).
	let customDefinitions: CompareMetricDefinition[] = $state([]);
	let removedKeys: string[] = $state([]);

	/**
	 * The built-in definitions first, then the metrics of both periods merged
	 * (common definitions deduplicated). Recomputed whenever the periods change.
	 */
	const automaticComparisonsPromise = $derived.by(async () => {
		// Read the reactive props synchronously so the derived tracks them.
		const firstPeriodName = firstPeriod.name;
		const secondPeriodName = secondPeriod.name;

		const [firstPeriodMetrics, secondPeriodMetrics] = await Promise.all([
			fetchTrainingPeriodMetrics(fetch, firstPeriod.id),
			fetchTrainingPeriodMetrics(fetch, secondPeriod.id)
		]);

		const defaultDefinitions = defaultCompareDefinitions();
		const defaultKeys = new Set(defaultDefinitions.map(definitionBaseKey));
		const periodDefinitions = mergedCompareDefinitions(
			firstPeriodMetrics,
			secondPeriodMetrics,
			firstPeriodName,
			secondPeriodName
		).filter((definition) => !defaultKeys.has(definitionBaseKey(definition)));

		return [...defaultDefinitions, ...periodDefinitions].map(buildMetricComparison);
	});

	const customComparisons = $derived(customDefinitions.map(buildMetricComparison));

	const addDefinition = (definition: CompareMetricDefinition) => {
		removedKeys = removedKeys.filter((key) => key !== definition.key);
		if (customDefinitions.some((candidate) => candidate.key === definition.key)) {
			return;
		}
		customDefinitions = [...customDefinitions, definition];
	};

	const removeDefinition = (key: string) => {
		customDefinitions = customDefinitions.filter((definition) => definition.key !== key);
		if (!removedKeys.includes(key)) {
			removedKeys = [...removedKeys, key];
		}
	};

	const buildPromise = async () => {
		const templates = await fetchTrainingMetricTemplates();
		return templates;
	};
	let metricTemplatesPromise = $state(buildPromise());
	let temporaryMetricName = $state('');
	let temporaryMetricFields: TrainingMetricFields = $state(emptyTrainingMetricFields());
	const fieldsToDefinition = (
		fields: TrainingMetricFields,
		name: string
	): Option<CompareMetricDefinition> => {
		const base = fieldsAsPayload(fields);
		return map(base, (b) => ({
			key: `custom:${name}`,
			label: name,
			source: 'default',
			base: b
		}));
	};

	let addMetricDialog: HTMLDialogElement;
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<h2 class="text-lg font-semibold">Training metrics</h2>
		<div class="flex flex-wrap items-center gap-2">
			<span class="text-sm opacity-70">Align by</span>
			<div class="join">
				<button
					class="btn join-item btn-sm"
					class:btn-active={alignBy === 'start'}
					onclick={() => (alignBy = 'start')}>Start</button
				>

				<button
					class="btn join-item btn-sm"
					class:btn-active={alignBy === 'end'}
					onclick={() => (alignBy = 'end')}>End</button
				>
			</div>
			<button class="btn btn-sm" onclick={() => addMetricDialog.show()}>
				<Plus class="size-4" />
				Add metric
			</button>
		</div>
	</div>
</div>

{#await automaticComparisonsPromise}
	<div class="mt-5 flex w-full flex-col items-center p-4">
		<div class="loading loading-bars"></div>
	</div>
{:then automaticComparisons}
	{@const comparisons = [
		...automaticComparisons.filter(
			(comparison) => !removedKeys.includes(comparison.definition.key)
		),
		...customComparisons
	]}
	{#each comparisons as comparison (comparison.definition.key)}
		<div class="mt-5">
			<CompareMetricEntry
				definition={comparison.definition}
				firstPeriod={{ period: firstPeriod, metric: comparison.firstPeriodValues }}
				secondPeriod={{ period: secondPeriod, metric: comparison.secondPeriodValues }}
				{alignBy}
				onRemove={() => removeDefinition(comparison.definition.key)}
			/>
		</div>
	{:else}
		<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
			<p class="text-sm tracking-wide italic opacity-70">
				No metric compared. Add a custom metric to compare these periods.
			</p>
		</div>
	{/each}
{/await}

<dialog class="modal" bind:this={addMetricDialog}>
	<div class="modal-box max-w-2xl">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<h3 class="mb-3 text-lg font-semibold">Create custom training metric</h3>
		<p class="mb-3 text-sm opacity-70">
			Each metric is computed over each period's own date range.
		</p>
		<fieldset class="fieldset rounded-box bg-base-100 p-2">
			{#await metricTemplatesPromise then metricTemplates}
				<TrainingMetricForm
					templates={metricTemplates}
					bind:fields={temporaryMetricFields}
					bind:metricName={temporaryMetricName}
				/>
				<div class="mt-4">
					<button
						class="btn w-full btn-neutral"
						onclick={() => {
							const definition = fieldsToDefinition(temporaryMetricFields, temporaryMetricName);
							if (isSome(definition)) {
								addDefinition(definition.value);
								temporaryMetricFields = emptyTrainingMetricFields();
								addMetricDialog.close();
							}
						}}
						disabled={fieldsAreEmpty(temporaryMetricFields)}
					>
						Add metric
					</button>
				</div>
			{/await}
		</fieldset>
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
