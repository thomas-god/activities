<script lang="ts">
	import ComparePeriodsSummary from '$ui/training_period/ComparePeriodsSummary.svelte';
	import NavbarPeriods from '$ui/navigation/NavbarPeriods.svelte';
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { dayjs } from '$lib/duration';
	import {
		fetchTrainingPeriodDetails,
		type TrainingPeriodDetails,
		type TrainingPeriodListItem
	} from '$lib/api';
	import { isNone, isSome, isSomeAnd, none, some, unwrapOr, type Option } from '$lib/Options';
	import { ArrowLeftRight } from '@lucide/svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let periods: TrainingPeriodListItem[] = $derived(
		data.periods.toSorted((a, b) => (a.start < b.start ? 1 : -1))
	);

	// The `periods` query parameter holds the two compared period ids positionally
	// as "first,second"; an empty segment means the slot has no period selected yet.
	const periodIdOption = (id: string | undefined): Option<string> =>
		id !== undefined && id !== '' ? some(id) : none();

	let urlIds: string[] = $derived((page.url.searchParams.get('periods') ?? '').split(','));
	let firstPeriodId: Option<string> = $derived(periodIdOption(urlIds[0]));
	let secondPeriodId: Option<string> = $derived(periodIdOption(urlIds[1]));

	const updateUrl = (first: Option<string>, second: Option<string>) => {
		const url = new URL(page.url);
		const param = isSome(second) ? `${unwrapOr(first, '')},${second.value}` : unwrapOr(first, '');
		if (param === '') {
			url.searchParams.delete('periods');
		} else {
			url.searchParams.set('periods', param);
		}
		/* eslint-disable svelte/no-navigation-without-resolve */
		goto(url, { replaceState: false, keepFocus: true });
	};

	const selectFirst = (periodId: string) => updateUrl(some(periodId), secondPeriodId);
	const selectSecond = (periodId: string) => updateUrl(firstPeriodId, some(periodId));

	const swapPeriods = () => updateUrl(secondPeriodId, firstPeriodId);
</script>

<NavbarPeriods invalidateTrainingPeriods={() => invalidate('app:training-periods')} />

{@render selectionSnippet()}

{#if isSome(firstPeriodId) && isSome(secondPeriodId)}
	{@render comparisonSnippet(firstPeriodId.value, secondPeriodId.value)}
{:else}
	<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
		<p class="text-sm tracking-wide italic opacity-70">Select a training period to compare.</p>
	</div>
{/if}

{#snippet selectionSnippet()}
	<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
		<h2 class="mb-3 text-lg font-semibold">Compare training periods</h2>
		<div class="flex flex-wrap items-center gap-2">
			{@render periodSelect('First period', firstPeriodId, secondPeriodId, selectFirst)}
			<button
				class="btn btn-sm"
				aria-label="Swap periods"
				disabled={isNone(firstPeriodId) || isNone(secondPeriodId)}
				onclick={swapPeriods}
			>
				<ArrowLeftRight class="size-4" />
			</button>
			{@render periodSelect('Second period', secondPeriodId, firstPeriodId, selectSecond)}
		</div>
	</div>
{/snippet}

{#snippet periodSelect(
	placeholder: string,
	selectedId: Option<string>,
	excludedId: Option<string>,
	onSelect: (periodId: string) => void
)}
	<select
		class="select w-64 max-w-full select-sm"
		aria-label={placeholder}
		value={unwrapOr(selectedId, '')}
		onchange={(e) => onSelect(e.currentTarget.value)}
	>
		<option value="" disabled>{placeholder}</option>
		{#each periods.filter((period) => !isSomeAnd(excludedId, (id) => id === period.id)) as period (period.id)}
			<option value={period.id}>
				{period.name} ({dayjs(period.start).format('MMM D, YYYY')} – {period.end === null
					? 'Ongoing'
					: dayjs(period.end).format('MMM D, YYYY')})
			</option>
		{:else}
			<option disabled class="italic">No training periods</option>
		{/each}
	</select>
{/snippet}

{#snippet comparisonSnippet(firstId: string, secondId: string)}
	{#await Promise.all( [fetchTrainingPeriodDetails(fetch, firstId), fetchTrainingPeriodDetails(fetch, secondId)] )}
		<div class="mt-5 flex w-full flex-col items-center p-4">
			<div class="loading loading-bars"></div>
		</div>
	{:then details}
		{@const loaded = details.filter((detail): detail is TrainingPeriodDetails => detail !== null)}
		{#if loaded.length === details.length}
			<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
				<h2 class="pb-1 text-lg font-semibold">Summary</h2>
				<ComparePeriodsSummary periods={loaded} />
			</div>
			{@render metricsPlaceholderSnippet()}
		{:else}
			<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
				<p class="text-sm tracking-wide italic opacity-80">
					Failed to load training period's details.
					<a class="link" href={resolve('/training/periods')}>Go back to periods.</a>
				</p>
			</div>
		{/if}
	{/await}
{/snippet}

{#snippet metricsPlaceholderSnippet()}
	<div class="mt-5 rounded-box bg-base-100 p-4 shadow-md">
		<h2 class="pb-1 text-lg font-semibold">Training metrics</h2>
		<div
			class="flex min-h-32 items-center justify-center rounded-box border-2 border-dashed border-base-300 p-6"
		>
			<p class="text-sm tracking-wide italic opacity-60">
				Training metric comparison charts coming soon
			</p>
		</div>
	</div>
{/snippet}
