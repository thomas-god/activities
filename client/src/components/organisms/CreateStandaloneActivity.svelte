<script lang="ts">
	import { goto } from '$app/navigation';
	import { postStandaloneActivity } from '$lib/api';
	import { sports } from '$lib/sport';
	import { dayjs } from '$lib/duration';

	let { activityCreatedCallback }: { activityCreatedCallback: () => void } = $props();

	let sport = $state<(typeof sports)[number]>('Running');
	let date = $state(dayjs().format('YYYY-MM-DDTHH:mm'));
	let durationHours = $state(0);
	let durationMinutes = $state(0);
	let distance = $state('');
	let elevation = $state('');
	let calories = $state('');

	let formState: 'NotSent' | 'Pending' | 'Success' | 'Error' | 'Duplicate' = $state('NotSent');

	let totalDurationSeconds = $derived(durationHours * 3600 + durationMinutes * 60);
	let canSubmit = $derived(totalDurationSeconds > 0 && date.trim() !== '');

	const handleSubmit = async () => {
		if (!canSubmit || formState === 'Pending') return;

		formState = 'Pending';

		const startTime = dayjs(date).format();

		const res = await postStandaloneActivity({
			start_time: startTime,
			duration: totalDurationSeconds,
			sport,
			distance: distance !== '' ? parseFloat(distance) * 1000 : undefined,
			elevation: elevation !== '' ? parseFloat(elevation) : undefined,
			calories: calories !== '' ? parseFloat(calories) : undefined
		});

		if (res.type === 'authentication-error') {
			goto('/login');
			return;
		}

		if (res.type === 'duplicate') {
			formState = 'Duplicate';
			return;
		}

		if (res.type === 'error') {
			formState = 'Error';
			return;
		}

		formState = 'Success';
		activityCreatedCallback();
	};
</script>

<fieldset class="fieldset rounded-box border-base-300 bg-base-100 px-0">
	<legend class="fieldset-legend text-base">Manually create an activity </legend>

	<div class="grid grid-cols-1 gap-x-4 gap-y-1 px-0 text-sm min-[500px]:grid-cols-2">
		<div>
			<label class="label" for="sa-sport">Sport</label>
			<select class="select w-full select-sm" id="sa-sport" bind:value={sport}>
				{#each sports as s}
					<option value={s}>{s}</option>
				{/each}
			</select>
		</div>

		<div>
			<label class="label" for="sa-date">Date & time</label>
			<input type="datetime-local" class="input input-sm w-full" id="sa-date" bind:value={date} />
		</div>

		<div>
			<label class="label" for="sa-duration-h">Duration</label>
			<div class="join">
				<input
					type="number"
					class="input input-sm join-item w-16"
					id="sa-duration-h"
					min="0"
					max="99"
					placeholder="h"
					bind:value={durationHours}
				/>
				<span class="join-item flex items-center bg-base-200 px-1">h</span>
				<input
					type="number"
					class="input input-sm join-item w-16"
					min="0"
					max="59"
					placeholder="min"
					bind:value={durationMinutes}
				/>
				<span class="join-item flex items-center bg-base-200 px-1">min</span>
			</div>
		</div>

		<div>
			<label class="label" for="sa-distance">Distance (km, optional)</label>
			<input
				type="number"
				class="input input-sm w-full"
				id="sa-distance"
				min="0"
				step="0.1"
				placeholder="e.g. 10.5"
				bind:value={distance}
			/>
		</div>

		<div>
			<label class="label" for="sa-elevation">Elevation (m, optional)</label>
			<input
				type="number"
				class="input input-sm w-full"
				id="sa-elevation"
				min="0"
				placeholder="e.g. 250"
				bind:value={elevation}
			/>
		</div>

		<div>
			<label class="label" for="sa-calories">Calories (kcal, optional)</label>
			<input
				type="number"
				class="input input-sm w-full"
				id="sa-calories"
				min="0"
				placeholder="e.g. 500"
				bind:value={calories}
			/>
		</div>
	</div>

	<button
		class="btn mt-3 w-full rounded-lg btn-sm btn-primary"
		disabled={!canSubmit || formState === 'Pending'}
		onclick={handleSubmit}
	>
		{#if formState === 'Pending'}
			<span class="loading loading-xs loading-spinner"></span>
		{:else}
			Create
		{/if}
	</button>

	{#if formState === 'Success'}
		<div class="mt-2 rounded-box bg-success/20 p-2 text-sm text-success-content">
			Activity created successfully!
		</div>
	{:else if formState === 'Duplicate'}
		<div class="mt-2 rounded-box bg-warning/20 p-2 text-sm text-warning-content">
			An activity with the same data already exists.
		</div>
	{:else if formState === 'Error'}
		<div class="mt-2 rounded-box bg-error/20 p-2 text-sm text-error-content">
			An error occurred, please try again.
		</div>
	{/if}
</fieldset>
