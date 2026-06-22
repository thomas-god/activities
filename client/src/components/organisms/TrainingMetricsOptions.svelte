<script lang="ts">
	import DateRange from '../molecules/DateRange.svelte';
	import MetricsOrderingDialog from './MetricsOrderingDialog.svelte';
	import { dayjs, localiseDate } from '$lib/duration';
	import type { TrainingMetricList, TrainingPeriodList } from '$lib/api';
	import type { MetricsOrderingScope } from '$lib/api/training-metrics-ordering';
	import { isNone, some, none, unwrapOr, type Option, isSomeAnd } from '$lib/Options';

	interface Props {
		dates: { start: string; end: string };
		datesUpdateCallback: (newDates: { start: string; end: string }) => void;
		periodsPromise?: Option<Promise<TrainingPeriodList>>;
		metricsOrderingScope: MetricsOrderingScope;
		metricsPromise: Option<Promise<TrainingMetricList>>;
		onMetricsReordered: () => void;
	}

	let {
		dates,
		datesUpdateCallback,
		periodsPromise = none(),
		metricsOrderingScope,
		metricsPromise,
		onMetricsReordered
	}: Props = $props();

	let metricsOrderingDialog: MetricsOrderingDialog;

	let selectedPeriodId: Option<string> = $state(none());
	let selectedQuickRange: Option<'4weeks' | '12weeks' | 'year'> = $state(none());

	const pastXWeeks = (numberOfWeek: number) => {
		selectedPeriodId = none();
		selectedQuickRange = some(numberOfWeek === 4 ? '4weeks' : '12weeks');
		const now = dayjs().startOf('day');
		const start = now.subtract(numberOfWeek, 'week');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};

	const thisYear = () => {
		selectedPeriodId = none();
		selectedQuickRange = some('year');
		const now = dayjs().startOf('day');
		const start = now.startOf('year');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};

	const selectPeriod = (periodId: string, periodStart: string, periodEnd: string | null) => {
		selectedPeriodId = some(periodId);
		selectedQuickRange = none();
		datesUpdateCallback({
			start: periodStart,
			end: periodEnd === null ? dayjs().toISOString() : periodEnd
		});
	};

	let sortedPeriods: Promise<TrainingPeriodList> = $derived.by(async () => {
		if (isNone(periodsPromise)) {
			return [];
		}
		return (await periodsPromise.value).toSorted((a, b) => (a.start < b.start ? 1 : -1));
	});
</script>

<div class="rounded-box border-base-300 bg-base-100 p-2 pt-4 shadow-md sm:p-4">
	<div class="flex items-center justify-between gap-2">
		<div class="pl-4">
			<DateRange
				bind:dates={
					() => dates,
					(d) => {
						selectedPeriodId = none();
						selectedQuickRange = none();
						datesUpdateCallback(d);
					}
				}
			/>
		</div>
	</div>
	<div class="flex flex-row flex-wrap items-center gap-2 py-2">
		<button
			class="btn btn-sm"
			class:btn-active={isSomeAnd(selectedQuickRange, (v) => v === '4weeks')}
			onclick={() => pastXWeeks(4)}>Last 4 weeks</button
		>
		<button
			class="btn btn-sm"
			class:btn-active={isSomeAnd(selectedQuickRange, (v) => v === '12weeks')}
			onclick={() => pastXWeeks(12)}>Last 12 weeks</button
		>
		<button
			class="btn btn-sm"
			class:btn-active={isSomeAnd(selectedQuickRange, (v) => v === 'year')}
			onclick={thisYear}>This year</button
		>
		{#await sortedPeriods then periods}
			<select
				class="select select-sm"
				value={unwrapOr(selectedPeriodId, '')}
				onchange={(e) => {
					const periodId = e.currentTarget.value;
					if (periodId) {
						const period = periods.find((p) => p.id === periodId);
						if (period) {
							selectPeriod(period.id, period.start, period.end);
						}
					}
				}}
			>
				<option value="" disabled selected>Training period</option>
				{#each periods as period}
					<option value={period.id}
						>{period.name} ({localiseDate(period.start)} - {period.end === null
							? 'Ongoing'
							: localiseDate(period.end)})</option
					>
				{:else}
					<option disabled class="italic">No training periods</option>
				{/each}
			</select>
		{/await}
	</div>
	<button onclick={() => metricsOrderingDialog.open()} class="btn btn-sm">
		<img src="/icons/order.svg" class="h-4 w-4" alt="List order icon" />
		Metrics order
	</button>
</div>

<MetricsOrderingDialog
	bind:this={metricsOrderingDialog}
	scope={metricsOrderingScope}
	metrics={await unwrapOr(metricsPromise, Promise.resolve([]))}
	onSaved={onMetricsReordered}
/>
