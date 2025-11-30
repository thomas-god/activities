<script lang="ts">
	import DateRange from '../molecules/DateRange.svelte';
	import MetricsOrderingDialog from './MetricsOrderingDialog.svelte';
	import { dayjs, localiseDate } from '$lib/duration';
	import type { TrainingPeriodList } from '$lib/api';
	import type { MetricsOrderingScope } from '$lib/api/training-metrics-ordering';

	interface Props {
		dates: { start: string; end: string };
		datesUpdateCallback: (newDates: { start: string; end: string }) => void;
		periods?: TrainingPeriodList;
		metricsOrderingScope: MetricsOrderingScope;
		metrics: Array<{
			id: string;
			name: string | null;
			granularity: string;
			aggregate: any;
			metric: string;
		}>;
		onMetricsReordered: () => void;
	}

	let {
		dates,
		datesUpdateCallback,
		periods = [],
		metricsOrderingScope,
		metrics,
		onMetricsReordered
	}: Props = $props();

	let metricsOrderingDialog: MetricsOrderingDialog;

	// Local state for DateRange component binding
	let localDates = $state({ start: dates.start, end: dates.end });

	// Update local dates when prop dates change
	$effect(() => {
		localDates = { start: dates.start, end: dates.end };
	});

	// Handle date changes from DateRange component
	$effect(() => {
		if (localDates.start !== dates.start || localDates.end !== dates.end) {
			selectedPeriodId = null; // Reset selected period when dates manually changed
			selectedQuickRange = null; // Reset quick range selection
			datesUpdateCallback(localDates);
		}
	});

	// Track selected training period
	let selectedPeriodId = $state<string | null>(null);
	let selectedQuickRange = $state<'4weeks' | '12weeks' | 'year' | null>(null);

	const pastXWeeks = (numberOfWeek: number) => {
		selectedPeriodId = null; // Reset selected period
		selectedQuickRange = numberOfWeek === 4 ? '4weeks' : '12weeks';
		const now = dayjs().startOf('day');
		const start = now.subtract(numberOfWeek, 'week');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};

	const thisYear = () => {
		selectedPeriodId = null; // Reset selected period
		selectedQuickRange = 'year';
		const now = dayjs().startOf('day');
		const start = now.startOf('year');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};

	const selectPeriod = (periodId: string, periodStart: string, periodEnd: string | null) => {
		selectedPeriodId = periodId;
		selectedQuickRange = null; // Reset quick range selection
		datesUpdateCallback({
			start: periodStart,
			end: periodEnd === null ? dayjs().toISOString() : periodEnd
		});
	};

	let sortedPeriods = $derived(periods.toSorted((a, b) => (a.start < b.start ? 1 : -1)));
</script>

<div class="rounded-box border-base-300 bg-base-100 p-2 pt-4 shadow-md sm:p-4">
	<div class="flex items-center justify-between gap-2">
		<div class="pl-4">
			<DateRange bind:dates={localDates} />
		</div>
	</div>
	<div class="flex flex-row flex-wrap items-center gap-2 py-2">
		<button
			class="btn btn-sm sm:btn-md"
			class:btn-active={selectedQuickRange === '4weeks'}
			onclick={() => pastXWeeks(4)}>Last 4 weeks</button
		>
		<button
			class="btn btn-sm sm:btn-md"
			class:btn-active={selectedQuickRange === '12weeks'}
			onclick={() => pastXWeeks(12)}>Last 12 weeks</button
		>
		<button
			class="btn btn-sm sm:btn-md"
			class:btn-active={selectedQuickRange === 'year'}
			onclick={thisYear}>This year</button
		>
		<select
			class="select select-sm"
			value={selectedPeriodId ?? ''}
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
			{#each sortedPeriods as period}
				<option value={period.id}
					>{period.name} ({localiseDate(period.start)} - {period.end === null
						? 'Ongoing'
						: localiseDate(period.end)})</option
				>
			{:else}
				<option disabled class="italic">No training periods</option>
			{/each}
		</select>
	</div>
	<button onclick={() => metricsOrderingDialog.open()} class="btn btn-ghost btn-sm">
		🔢 Metrics order
	</button>
</div>

<MetricsOrderingDialog
	bind:this={metricsOrderingDialog}
	scope={metricsOrderingScope}
	{metrics}
	onSaved={onMetricsReordered}
/>
