<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { type Sport, type SportCategory } from '$lib/sport';
	import DateRange from '../molecules/DateRange.svelte';
	import SportsSelect from '../molecules/SportsSelect.svelte';

	let { callback }: { callback: () => void } = $props();

	let name = $state('');
	let dates = $state({ start: null, end: null });
	let datesValid = $derived(dates.start !== null);
	let note = $state('');

	let selectedSports: Sport[] = $state([]);
	let selectedSportCategories: SportCategory[] = $state([]);
	let sportFilterSelected = $state(false);

	let requestPending = $state(false);

	let missingInformation = $derived(!datesValid || name === '');

	let periodRequest = $derived.by(() => {
		let basePayload = {
			start: dates.start,
			end: dates.end,
			name,
			sports: null as null | ({ Sport: Sport } | { SportCategory: SportCategory })[],
			note: note.trim() === '' ? null : note.trim()
		};

		if (sportFilterSelected) {
			const sportFilter = selectedSports.map((sport) => ({
				Sport: sport
			}));
			const sportCategoriesFilter = selectedSportCategories.map((category) => ({
				SportCategory: category
			}));
			const filters: ({ Sport: Sport } | { SportCategory: SportCategory })[] = [
				...sportFilter,
				...sportCategoriesFilter
			];
			basePayload = { ...basePayload, sports: filters };
		}

		return basePayload;
	});

	const createPeriodCallback = async (payload: Object): Promise<void> => {
		const body = JSON.stringify(payload);
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/period`, {
			body,
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto('/login');
		}
		invalidate('app:training-metrics');
		callback();
	};
</script>

<div class=" text-sm">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<legend class="fieldset-legend text-base">New training period</legend>
		<label class="label" for="period-name">Training period name</label>
		<input type="text" placeholder="Name" class="input" id="period-name" bind:value={name} />

		<label class="label" for="period-dates">Training period dates</label>
		<DateRange bind:dates />

		<label class="label" for="period-note">Note (optional)</label>
		<textarea
			id="period-note"
			class="textarea-bordered textarea"
			placeholder="Add notes about this training period..."
			rows="3"
			bind:value={note}
		></textarea>

		<div class="my-2 font-semibold">
			<span class="pr-2"> Filter by sports </span>
			<input type="checkbox" bind:checked={sportFilterSelected} class="toggle toggle-sm" />
		</div>
		{#if sportFilterSelected}
			<SportsSelect bind:selectedSports bind:selectedSportCategories />
		{/if}

		<button
			class="btn mt-4 btn-neutral"
			onclick={async () => {
				requestPending = true;
				await createPeriodCallback(periodRequest);
				requestPending = false;
			}}
			disabled={requestPending || missingInformation}
			>Create period
			{#if requestPending}
				<span class="loading loading-spinner"></span>
			{/if}
		</button>
	</fieldset>
</div>
