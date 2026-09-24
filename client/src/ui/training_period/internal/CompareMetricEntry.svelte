<script lang="ts">
	import * as d3 from 'd3';
	import { X } from '@lucide/svelte';
	import type { TrainingMetric, TrainingPeriodDetails } from '$lib/api';
	import { asOption, isSome, none, some, type Option } from '$lib/Options';
	import {
		definitionLabel,
		type CompareAlignment,
		type CompareMetricDefinition
	} from '$lib/trainingMetric';
	import TrainingMetricChart, {
		type ChartHandle,
		type HoveredBin
	} from '$ui/training_metrics/TrainingMetricChart.svelte';
	import { computeExtendedTimeDomain, computeMissingNumberOfBins } from './CompareMetricEntry';
	import { dayjs } from '$lib/duration';

	interface ComparedSide {
		period: TrainingPeriodDetails;
		metric: Promise<Option<TrainingMetric>>;
	}

	let {
		definition,
		firstPeriod,
		secondPeriod,
		alignBy,
		onRemove
	}: {
		definition: CompareMetricDefinition;
		firstPeriod: ComparedSide;
		secondPeriod: ComparedSide;
		alignBy: CompareAlignment;
		onRemove: () => void;
	} = $props();

	let chartWidths: number[] = $state([300, 300]);

	let firstChartRef = $state<ChartHandle>();
	let secondChartRef = $state<ChartHandle>();

	let globalMax = $derived(Math.max(firstChartRef?.getYMax() ?? 0, secondChartRef?.getYMax() ?? 0));
	let hoveredBin: Option<HoveredBin> = $state(none());

	const heightFor = (width: number): number => Math.max(150, Math.min(300, width * 0.6));
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="mb-2 flex items-center justify-between gap-2">
		<h3 class="text-base font-semibold">
			{definitionLabel(definition)}
			{#if definition.source !== 'default'}
				<span class="badge badge-ghost align-middle badge-sm">from: {definition.source}</span>
			{/if}
		</h3>
		<button class="btn btn-ghost btn-xs" aria-label="Remove metric comparison" onclick={onRemove}>
			<X class="size-4" />
		</button>
	</div>

	<div class="grid grid-cols-1 gap-4 min-[700px]:grid-cols-2">
		{#await Promise.all([firstPeriod.metric, secondPeriod.metric])}
			<div class="col-span-1 flex w-full items-center justify-center p-8 min-[700px]:col-span-2">
				<div class="loading loading-bars"></div>
			</div>
		{:then [firstMetricOpt, secondMetricOpt]}
			{#if isSome(firstMetricOpt) && isSome(secondMetricOpt)}
				{@const firstMetric = firstMetricOpt.value}
				{@const secondMetric = secondMetricOpt.value}
				{@const deltas = computeMissingNumberOfBins(
					{ metric: firstMetric, period: firstPeriod.period },
					{ metric: secondMetric, period: secondPeriod.period }
				)}
				{@const sides = [
					{ period: firstPeriod.period, metric: firstMetric, delta: deltas.first, type: 'first' },
					{
						period: secondPeriod.period,
						metric: secondMetric,
						delta: deltas.second,
						type: 'second'
					}
				]}
				{#each sides as side, idx (side.period.id)}
					{@const timeDomain = computeExtendedTimeDomain(
						side.period,
						side.delta,
						asOption(side.metric.granularity),
						alignBy,
						dayjs()
					)}
					<div class="flex flex-col" bind:clientWidth={chartWidths[idx]}>
						<div class="mb-1 flex items-center gap-1.5 text-sm">
							<span
								class="inline-block h-2.5 w-2.5 rounded-full"
								style="background-color: {d3.schemeTableau10[idx % d3.schemeTableau10.length]}"
							></span>
							{side.period.name}
						</div>
						{#if side.metric === undefined}
							<div class="p-3 text-center text-sm tracking-wide italic opacity-60">
								Failed to compute metric
							</div>
						{:else}
							<TrainingMetricChart
								bind:this={
									() => (side.type === 'first' ? firstChartRef : secondChartRef),
									(v) => {
										if (side.type === 'first') {
											firstChartRef = v;
										} else {
											secondChartRef = v;
										}
									}
								}
								metric={side.metric}
								width={chartWidths[idx]}
								height={heightFor(chartWidths[idx])}
								{timeDomain}
								displayMode="relative"
								yMax={some(globalMax)}
								bind:syncHoveredBin={hoveredBin}
							/>
						{/if}
					</div>
				{/each}
			{/if}
		{/await}
	</div>
</div>
