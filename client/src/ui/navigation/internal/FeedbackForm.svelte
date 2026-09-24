<script lang="ts">
	import { fetchHooperIndex, saveHooperIndex, type HooperIndex } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { isSome, none, some, type Option } from '$lib/Options';
	import { ChevronLeft, ChevronRight } from '@lucide/svelte';
	import {
		emptyHooperIndex,
		HOOPER_MEASURE_MAX,
		HOOPER_MEASURE_MIN,
		hooperMeasures,
		type HooperMeasure
	} from './feedback';

	let { callback = () => {} }: { callback?: () => void } = $props();

	let date = $state(dayjs().format('YYYY-MM-DD'));
	const setLoadPromise = () =>
		Promise.all([
			fetchHooperIndex(date),
			fetchHooperIndex(dayjs(date).subtract(1, 'day').format('YYYY-MM-DD'))
		])
			.then(([loaded, previous]) => {
				values = { ...loaded };
				baseline = { ...loaded };
				previousValues = { ...previous };
				loadError = false;
			})
			.catch(() => {
				loadError = true;
			});
	let loadPromise = $derived(setLoadPromise());
	let loadError = $derived(false);
	let values = $state<HooperIndex>(emptyHooperIndex());
	let baseline = $state<HooperIndex>(emptyHooperIndex());
	let previousValues = $state<HooperIndex>(emptyHooperIndex());
	const isToday = $derived(date === dayjs().format('YYYY-MM-DD'));
	const previousLabel = $derived(isToday ? 'yesterday' : 'day before');
	const isDirty = $derived(hooperMeasures.some((measure) => values[measure] !== baseline[measure]));

	let savePromise: Option<Promise<void>> = $state(none());
	const save = () =>
		(savePromise = some(
			saveHooperIndex(date, { ...values }).then((_) => {
				setLoadPromise();
				callback();
			})
		));
	const setMeasure = (measure: HooperMeasure, value: number | null) => {
		values = { ...values, [measure]: value };
	};

	const clear = () => (values = emptyHooperIndex());
	const reset = () => (values = { ...baseline });
</script>

<fieldset class="fieldset rounded-box border-base-300 bg-base-100">
	<legend class="fieldset-legend text-base">Update subjective feedback</legend>
	<div class="flex flex-row items-center gap-1">
		<label class="label mr-3" for="hooper-date">Date</label>
		<button
			class="btn btn-sm"
			onclick={() => (date = dayjs(date).subtract(1, 'day').format('YYYY-MM-DD'))}
		>
			<ChevronLeft class="size-4" />
		</button>
		<input id="hooper-date" type="date" class="input w-35 input-sm" bind:value={date} />
		<button
			class="btn btn-sm"
			onclick={() => (date = dayjs(date).add(1, 'day').format('YYYY-MM-DD'))}
		>
			<ChevronRight class="size-4" />
		</button>
	</div>

	{#if loadError}
		<p class="text-error">Failed to load the values for this date.</p>
	{:else}
		{#each hooperMeasures as measure (measure)}
			{@const value = values[measure]}
			<div class="flex flex-col gap-1">
				<div class="flex items-center justify-between">
					<label class="label capitalize" for="hooper-{measure}">
						{measure}
						<span class="text-xs font-normal normal-case opacity-50">
							({previousLabel}: {previousValues[measure] ?? '–'} )
						</span>
					</label>
					<div class="flex items-center gap-1">
						<span
							class="text-sm font-semibold tabular-nums"
							class:opacity-40={value === null}
							data-testid="hooper-{measure}-value"
						>
							{value ?? 'Not set'}
						</span>
						<button
							class="btn btn-ghost px-1 btn-xs"
							class:invisible={value === null}
							aria-label="Clear {measure}"
							onclick={() => setMeasure(measure, null)}>✕</button
						>
					</div>
				</div>

				<div class="flex items-center gap-2">
					<span class="w-3 text-end text-xs tabular-nums opacity-40">{HOOPER_MEASURE_MIN}</span>
					<input
						id="hooper-{measure}"
						type="range"
						min={HOOPER_MEASURE_MIN}
						max={HOOPER_MEASURE_MAX}
						step="1"
						class="range flex-1 range-neutral range-sm"
						class:range-empty={value === null}
						value={value ?? HOOPER_MEASURE_MIN}
						oninput={(event) =>
							setMeasure(measure, Number((event.currentTarget as HTMLInputElement).value))}
					/>
					<span class="w-3 text-xs tabular-nums opacity-40">{HOOPER_MEASURE_MAX}</span>
				</div>
			</div>
		{/each}

		{@render actions()}
	{/if}
	{#await loadPromise}{/await}
</fieldset>

{#snippet actions()}
	<div class="mt-2 flex flex-wrap gap-2">
		{#if isSome(savePromise)}
			{#await savePromise.value}
				<button class="btn btn-primary btn-sm" disabled
					>Save <span class="loading loading-sm"></span>
				</button>
			{:then}
				<button class="btn btn-primary btn-sm" disabled={!isDirty} onclick={save}>Save</button>
			{/await}
		{:else}
			<button class="btn btn-primary btn-sm" disabled={!isDirty} onclick={save}>Save</button>
		{/if}
		<button class="btn btn-ghost btn-sm" disabled={!isDirty} onclick={reset}>Reset</button>
		<button class="btn btn-ghost btn-sm" onclick={clear}>Clear</button>
	</div>
{/snippet}

<style>
	/* An unset measure shows an empty track: no filled progress and no thumb.
	   Any interaction with the slider sets the measure. */
	.range-empty {
		--range-fill: 0;
	}

	.range-empty::-webkit-slider-thumb {
		opacity: 0;
	}

	.range-empty::-moz-range-thumb {
		opacity: 0;
	}
</style>
