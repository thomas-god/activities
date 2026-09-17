<script lang="ts">
	import {
		fetchWeightAndNutrition,
		saveWeightAndNutrition,
		type UpdateWeightAndNutritionPatch,
		type WeightAndNutrition
	} from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { isSome, none, some, type Option } from '$lib/Options';
	import { GlassWater, Utensils, WeightTilde } from '@lucide/svelte';
	import {
		emptyWeightAndNutrition,
		weightAndNutritionCategories,
		weightAndNutritionLabels,
		weightAndNutritionMeasures,
		weightAndNutritionUnits,
		type WeightAndNutritionCategory,
		type WeightAndNutritionMeasure
	} from '.';

	/** Inputs are kept as raw strings so partially typed decimals ("70.") survive re-renders. */
	type EditableValues = Record<WeightAndNutritionMeasure, string>;

	const toEditable = (values: WeightAndNutrition): EditableValues =>
		Object.fromEntries(
			weightAndNutritionMeasures.map((measure) => [
				measure,
				values[measure] === null ? '' : String(values[measure])
			])
		) as EditableValues;

	const parseMeasure = (raw: string): number | null => {
		const trimmed = raw.trim();
		if (trimmed === '') return null;
		const value = Number(trimmed);
		return Number.isFinite(value) ? value : null;
	};

	const toPatch = (values: EditableValues): UpdateWeightAndNutritionPatch =>
		Object.fromEntries(
			weightAndNutritionMeasures.map((measure) => [measure, parseMeasure(values[measure])])
		) as UpdateWeightAndNutritionPatch;

	let date = $state(dayjs().format('YYYY-MM-DD'));
	const setLoadPromise = () =>
		fetchWeightAndNutrition(date).then((loaded) => {
			values = toEditable(loaded);
			baseline = toEditable(loaded);
		});
	let loadPromise = $derived(setLoadPromise());
	let values = $state<EditableValues>(toEditable(emptyWeightAndNutrition()));
	let baseline = $state<EditableValues>(toEditable(emptyWeightAndNutrition()));
	const isDirty = $derived(
		weightAndNutritionMeasures.some(
			(measure) => parseMeasure(values[measure]) !== parseMeasure(baseline[measure])
		)
	);

	let savePromise: Option<Promise<void>> = $state(none());
	const save = () =>
		(savePromise = some(
			saveWeightAndNutrition(date, toPatch(values)).then((_) => setLoadPromise())
		));
	const setMeasure = (measure: WeightAndNutritionMeasure, value: string) => {
		values = { ...values, [measure]: value };
	};

	const clear = () => (values = toEditable(emptyWeightAndNutrition()));
	const reset = () => (values = { ...baseline });
</script>

<fieldset class="fieldset rounded-box border-base-300 bg-base-100">
	<legend class="fieldset-legend text-base">Update weight &amp; nutrition</legend>
	<div class="flex flex-row gap-2">
		<label class="label" for="wn-date">Date</label>
		<input id="wn-date" type="date" class="input w-full input-sm" bind:value={date} />
	</div>

	{#await loadPromise}
		<div class="flex justify-center py-6">
			<span class="loading loading-lg loading-spinner"></span>
		</div>
	{:then}
		<div class="flex flex-col gap-3">
			{#each weightAndNutritionCategories as category (category.key)}
				<div class="rounded-box border border-base-300 p-3">
					<p class="mb-1 flex flex-row items-center gap-2 text-sm font-semibold">
						{@render categoryIcon(category.key)}
						{category.label}
					</p>
					{@render measureField(category.main)}

					{#if category.subs.length > 0}
						<details class="collapse-arrow collapse mt-1" data-testid="wn-{category.key}-details">
							<summary class="collapse-title min-h-0 pt-1 pb-2 pl-1 text-xs italic opacity-70">
								More
							</summary>
							<div class="collapse-content flex flex-col gap-2 px-0">
								{#each category.subs as sub (sub)}
									{@render measureField(sub)}
								{/each}
							</div>
						</details>
					{/if}
				</div>
			{/each}
		</div>

		{@render actions()}
	{:catch}
		<p class="text-error">Failed to load the values for this date.</p>
	{/await}
</fieldset>

{#snippet measureField(measure: WeightAndNutritionMeasure)}
	{@const value = values[measure]}
	{@const unit = weightAndNutritionUnits[measure]}
	{@const label = weightAndNutritionLabels[measure]}
	<div class="flex items-end gap-2">
		<div class="flex flex-1 flex-row gap-2">
			<label class="label w-22 shrink-0 py-1" for="wn-{measure}">
				{label}{#if unit}
					<span class="text-xs opacity-50">({unit})</span>{/if}
			</label>
			<input
				id="wn-{measure}"
				type="number"
				step="any"
				class="input w-full input-sm"
				placeholder="Not set"
				{value}
				oninput={(event) => setMeasure(measure, (event.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<button
			class="btn mb-0.5 btn-ghost px-1 btn-xs"
			class:invisible={value === ''}
			aria-label="Clear {label}"
			onclick={() => setMeasure(measure, '')}>✕</button
		>
	</div>
{/snippet}

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

{#snippet categoryIcon(category: WeightAndNutritionCategory['key'])}
	{#if category === 'weight'}
		<WeightTilde class="size-5" />
	{:else if category === 'nutrition'}
		<Utensils class="size-5" />
	{:else if category === 'hydration'}
		<GlassWater class="size-5" />
	{/if}
{/snippet}
