<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { untrack } from 'svelte';
	import {
		fetchTrainingPeriodMetrics,
		getTrainingMetricPreview,
		type TrainingMetric,
		type TrainingMetricList,
		type TrainingPeriodDetails
	} from '$lib/api';
	import { type Option } from '$lib/Options';
	import {
		definitionLabel,
		metricPreviewPayload,
		defaultCompareDefinitions,
		metricDefinitionKey,
		extractBaseDefinitionFromMetric,
		type CompareAlignment,
		type CompareMetricDefinition
	} from '$lib/trainingMetric';
	import CompareMetricEntry from './internal/CompareMetricEntry.svelte';

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

	// The computation promises are created here, when a definition is added, and
	// in the effect below when the compared periods change — never during
	// rendering, which Svelte forbids (state_unsafe_mutation).
	let comparisons: MetricComparison[] = $state(
		defaultCompareDefinitions().map(buildMetricComparison)
	);

	// The ids are snapshotted on purpose: they record which periods the current
	// promises were built for, and the effect below rebuilds when they change.
	// svelte-ignore state_referenced_locally
	let builtForFirstId: string | null = firstPeriod.id;
	// svelte-ignore state_referenced_locally
	let builtForSecondId: string | null = secondPeriod.id;

	$effect(() => {
		if (firstPeriod.id === builtForFirstId && secondPeriod.id === builtForSecondId) {
			return;
		}
		builtForFirstId = firstPeriod.id;
		builtForSecondId = secondPeriod.id;
		untrack(() => {
			comparisons = comparisons.map((comparison) => buildMetricComparison(comparison.definition));
		});
	});

	let addMetricDialog: HTMLDialogElement;

	let firstPeriodMetricsPromise = $derived(fetchTrainingPeriodMetrics(fetch, firstPeriod.id));
	let secondPeriodMetricsPromise = $derived(fetchTrainingPeriodMetrics(fetch, secondPeriod.id));

	let endAlignable = $derived(firstPeriod.end !== null && secondPeriod.end !== null);

	// The End alignment only applies while both periods have an end date; fall
	// back to Start otherwise (e.g. after selecting an ongoing period), so the
	// charts never mix a start anchor with end-style labeling.
	let effectiveAlignBy = $derived.by((): CompareAlignment => (endAlignable ? alignBy : 'start'));

	const addDefinition = (definition: CompareMetricDefinition) => {
		if (comparisons.some((comparison) => comparison.definition.key === definition.key)) {
			return;
		}
		comparisons = [...comparisons, buildMetricComparison(definition)];
	};

	const removeDefinition = (key: string) => {
		comparisons = comparisons.filter((comparison) => comparison.definition.key !== key);
	};

	type PickerEntry = { definition: CompareMetricDefinition; added: boolean };

	const pickerEntries = (
		firstPeriodMetrics: TrainingMetricList,
		firstPeriodName: string,
		secondPeriodMetrics: TrainingMetricList,
		secondPeriodName: string
	): PickerEntry[] => {
		const picked: {
			key: string;
			source: string;
			metric: TrainingMetric;
		}[] = [];

		for (const metric of firstPeriodMetrics) {
			if (metric.granularity === null) continue;
			picked.push({ key: metricDefinitionKey(metric), source: firstPeriodName, metric });
		}
		for (const metric of secondPeriodMetrics) {
			if (metric.granularity === null) continue;
			const key = metricDefinitionKey(metric);
			const existing = picked.find((entry) => entry.key === key);
			if (existing !== undefined) {
				existing.source = 'both';
			} else {
				picked.push({ key, source: secondPeriodName, metric });
			}
		}

		return picked
			.map(({ key, source, metric }) => ({
				definition: {
					key,
					label: metric.name,
					source,
					base: extractBaseDefinitionFromMetric(metric)
				},
				added: comparisons.some((comparison) => comparison.definition.key === key)
			}))
			.toSorted((a, b) =>
				definitionLabel(a.definition).localeCompare(definitionLabel(b.definition))
			);
	};
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<h2 class="text-lg font-semibold">Training metrics</h2>
		<div class="flex flex-wrap items-center gap-2">
			<span class="text-sm opacity-70">Align by</span>
			<div class="join">
				<button
					class="btn join-item btn-sm"
					class:btn-active={effectiveAlignBy === 'start'}
					onclick={() => (alignBy = 'start')}>Start</button
				>
				<div
					class="tooltip"
					data-tip={endAlignable
						? 'Align periods by their end date'
						: 'Unavailable while a period is ongoing'}
				>
					<button
						class="btn join-item btn-sm"
						class:btn-active={effectiveAlignBy === 'end'}
						disabled={!endAlignable}
						onclick={() => (alignBy = 'end')}>End</button
					>
				</div>
			</div>
			<button class="btn btn-sm" onclick={() => addMetricDialog.show()}>
				<Plus class="size-4" />
				Add metric
			</button>
		</div>
	</div>
</div>

{#each comparisons as comparison (comparison.definition.key)}
	<div class="mt-5">
		<CompareMetricEntry
			definition={comparison.definition}
			firstPeriod={{ period: firstPeriod, metric: comparison.firstPeriodValues }}
			secondPeriod={{ period: secondPeriod, metric: comparison.secondPeriodValues }}
			alignBy={effectiveAlignBy}
			onRemove={() => removeDefinition(comparison.definition.key)}
		/>
	</div>
{:else}
	<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
		<p class="text-sm tracking-wide italic opacity-70">
			No metric compared. Add one from the training metrics of the periods.
		</p>
	</div>
{/each}

<dialog class="modal" bind:this={addMetricDialog}>
	<div class="modal-box max-w-2xl">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<h3 class="mb-3 text-lg font-semibold">Add a metric comparison</h3>
		<p class="mb-3 text-sm opacity-70">
			Each metric is computed over each period's own date range for the comparison.
		</p>
		{#await Promise.all([firstPeriodMetricsPromise, secondPeriodMetricsPromise])}
			<div class="flex justify-center p-4">
				<div class="loading loading-bars"></div>
			</div>
		{:then [firstPeriodMetrics, secondPeriodMetrics]}
			<ul class="menu w-full rounded-box bg-base-200 p-2">
				{#each pickerEntries(firstPeriodMetrics, firstPeriod.name, secondPeriodMetrics, secondPeriod.name) as entry (entry.definition.key)}
					<li class="w-full" class:disabled={entry.added}>
						<button disabled={entry.added} onclick={() => addDefinition(entry.definition)}>
							<div class="flex w-full flex-col items-start gap-0.5">
								<span>
									{definitionLabel(entry.definition)}
									<span class="badge badge-ghost badge-xs">{entry.definition.source}</span>
								</span>
								<span class="text-xs opacity-60">
									{entry.definition.base.metric}
									· {entry.definition.base.window?.granularity}
									{entry.definition.base.window?.aggregate}
									{#if entry.definition.base.window?.group_by}
										· by {entry.definition.base.window.group_by}
									{/if}
								</span>
							</div>
							{#if entry.added}
								<span class="badge badge-ghost badge-sm">Added</span>
							{/if}
						</button>
					</li>
				{:else}
					<li class="disabled">
						<span>No comparable training metrics in these periods</span>
					</li>
				{/each}
			</ul>
		{/await}
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
