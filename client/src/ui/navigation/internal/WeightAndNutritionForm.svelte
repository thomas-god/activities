<script lang="ts">
	import {
		fetchWeightAndNutrition,
		importWeightAndNutritionHistory,
		saveWeightAndNutrition,
		type UpdateWeightAndNutritionPatch,
		type WeightAndNutrition
	} from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { isSome, none, some, type Option } from '$lib/Options';
	import {
		ArchiveRestore,
		CalendarArrowUp,
		ChevronLeft,
		ChevronRight,
		GlassWater,
		RotateCcw,
		Save,
		Utensils,
		WeightTilde
	} from '@lucide/svelte';
	import {
		emptyWeightAndNutrition,
		weightAndNutritionCategories,
		weightAndNutritionLabels,
		weightAndNutritionMeasures,
		weightAndNutritionUnits,
		type WeightAndNutritionCategory,
		type WeightAndNutritionMeasure
	} from './weightAndNutrition';
	import FormTitle from '../FormTitle.svelte';

	let {
		callback = () => {},
		next = none(),
		previous = none()
	}: { callback?: () => void; next?: Option<() => void>; previous?: Option<() => void> } = $props();

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
	const isToday = $derived(date === dayjs().format('YYYY-MM-DD'));
	const setLoadPromise = () =>
		Promise.all([
			fetchWeightAndNutrition(date),
			fetchWeightAndNutrition(dayjs(date).subtract(1, 'day').format('YYYY-MM-DD'))
		])
			.then(([loaded, previous]) => {
				values = toEditable(loaded);
				baseline = toEditable(loaded);
				previousValues = toEditable(previous);
				loadError = false;
			})
			.catch(() => {
				loadError = true;
			});
	let loadPromise = $derived(setLoadPromise());
	let loadError = $state(false);
	let values = $state<EditableValues>(toEditable(emptyWeightAndNutrition()));
	let previousValues = $state<EditableValues>(toEditable(emptyWeightAndNutrition()));
	let baseline = $state<EditableValues>(toEditable(emptyWeightAndNutrition()));
	const isDirty = $derived(
		weightAndNutritionMeasures.some(
			(measure) => parseMeasure(values[measure]) !== parseMeasure(baseline[measure])
		)
	);

	let savePromise: Option<Promise<void>> = $state(none());
	const save = () =>
		(savePromise = some(
			saveWeightAndNutrition(date, toPatch(values)).then((_) => {
				setLoadPromise();
				callback();
			})
		));
	const setMeasure = (measure: WeightAndNutritionMeasure, value: string) => {
		values = { ...values, [measure]: value };
	};

	let showHistoryImport = $state(false);
	let files: FileList | undefined = $state(undefined);
	let fileUploadContent = $state('');
	let canUpload = $derived(files !== undefined && (files as FileList).length > 0);
	let uploadPromise: Option<Promise<void>> = $state(none());
	let fileFailures: { file: string; reason: string }[] = $derived([]);
	let uploadError: Option<string> = $derived(none());
	let uploadSuccess = $state(false);

	const setImportHistoryPromise = async () => {
		if (!canUpload) {
			return;
		}
		uploadPromise = some(
			importWeightAndNutritionHistory(files as FileList).then((err) => {
				if (err.type === 'totalFailure') {
					uploadError = some(err.reason);
					fileFailures = [];
					uploadSuccess = false;
				} else if (err.type === 'partialFailure') {
					uploadError = none();
					fileFailures = err.files;
					uploadSuccess = false;
				} else if (err.type === 'success') {
					uploadError = none();
					fileFailures = [];
					uploadSuccess = true;
				}
			})
		);
	};

	const reset = () => (values = { ...baseline });

	const inputStepValue = (unit: string): number => {
		if (unit === 'kg' || unit === 'L') {
			return 0.1;
		} else if (unit === 'g') {
			return 10;
		} else if (unit === 'kcal') {
			return 100;
		} else {
			return 1;
		}
	};
</script>

{#snippet content()}
	<Utensils class="size-4" />
	Weight &amp; nutrition
{/snippet}

<fieldset class="fieldset min-w-0 rounded-box border-base-300 bg-base-100">
	<FormTitle {previous} {next} {content} />

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

	{#await loadPromise}{/await}
	{#if !loadError}
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
		{#if showHistoryImport}
			<div class="min-w-0 rounded-box border border-base-300 p-3">
				<p class="mb-1 flex flex-row items-center gap-2 text-sm font-semibold">
					<ArchiveRestore class="size-5" />
					Import history
				</p>
				<p class="pb-1 italic">
					You can import your history by uploading one or more csv files of the form (omitting
					columns for values you don't want to import):
				</p>
				<pre
					class="overflow-x-auto rounded-box border border-base-300 bg-base-200 p-2 font-mono text-xs leading-relaxed">{`date,weight,fat,muscle,calories,lipid,carbs,protein,water,alcohol
2024-01-15,70.5,15.2,55.1,2000,60,250,120,1.5,0.5`}</pre>
				<div class="join mt-2 gap-3">
					<input
						type="file"
						class="file-input"
						accept=".csv,.csv.gz"
						multiple
						bind:files
						bind:value={fileUploadContent}
						id="activity_file"
						name="activity file"
					/>
					{#if isSome(uploadPromise)}
						{#await uploadPromise.value}
							<button class="btn rounded-lg btn-primary" disabled>
								Upload <span class="loading loading-spinner"></span>
							</button>
						{:then}
							<!-- TODO: display something in case of success, else no feedback -->
							<button
								class="btn rounded-lg btn-primary"
								disabled={!canUpload}
								onclick={setImportHistoryPromise}
							>
								Upload
							</button>
						{/await}
					{:else}
						<button
							class="btn rounded-lg btn-primary"
							disabled={!canUpload}
							onclick={setImportHistoryPromise}
						>
							Upload
						</button>
					{/if}
				</div>
				{#if uploadSuccess}
					<div class="mt-2 rounded-box bg-success/20 p-3 text-success-content">
						History import successful
					</div>
				{/if}
				{#if fileFailures.length > 0}
					<div class="mt-2 rounded-box bg-error/20 p-3 text-error-content">
						Some files ({fileFailures.length}) could not be processed
						<ul>
							{#each fileFailures as { file, reason } (file)}
								<li>
									{file}: {reason}
								</li>
							{/each}
						</ul>
					</div>
				{/if}
				{#if isSome(uploadError)}
					<p class="mt-2 rounded-box bg-error/20 p-3 text-error-content">
						Error while importing history: {uploadError.value}
					</p>
				{/if}
			</div>
		{/if}
		{@render actions()}
	{:else}
		<p class="text-error">Failed to load the values for this date.</p>
	{/if}
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
				step={inputStepValue(unit)}
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
				<button class="btn btn-primary btn-sm" disabled>
					<Save class="size-4" />
					Save <span class="loading loading-sm"></span>
				</button>
			{:then}
				<button class="btn btn-primary btn-sm" disabled={!isDirty} onclick={save}>
					<Save class="size-4" />
					Save
				</button>
			{/await}
		{:else}
			<button class="btn btn-primary btn-sm" disabled={!isDirty} onclick={save}>
				<Save class="size-4" />
				Save
			</button>
		{/if}
		<button class="btn btn-ghost btn-sm" disabled={!isDirty} onclick={reset}>
			<RotateCcw class="size-4" />
			Reset
		</button>
		<!-- <button class="btn btn-ghost btn-sm" onclick={clear}>
			<CircleX class="size-4" />
			Clear
		</button> -->
		<button class="btn btn-ghost btn-sm" onclick={() => (values = previousValues)}>
			<CalendarArrowUp class="size-4" />
			Values from {isToday ? 'yesterday' : 'the day before'}
		</button>
		<button class="btn btn-ghost btn-sm" onclick={() => (showHistoryImport = !showHistoryImport)}>
			<ArchiveRestore class="size-4" />
			{showHistoryImport ? 'Close history import' : 'Import history'}
		</button>
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
