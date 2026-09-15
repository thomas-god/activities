<script lang="ts">
	import {
		createHooperIndex,
		deleteHooperIndex,
		updateHooperIndex,
		type HooperIndex
	} from '$lib/api';
	import { dayjs } from '$lib/duration';

	// Temporary page used to manually exercise the Hooper index endpoints.
	// Not linked from anywhere and not wired to the rest of the app.

	const measures = ['fatigue', 'sleep', 'pain', 'stress', 'mood'] as const;
	type Measure = (typeof measures)[number];

	let date = $state(dayjs().format('YYYY-MM-DD'));
	let values: Record<Measure, number | null> = $state({
		fatigue: null,
		sleep: null,
		pain: null,
		stress: null,
		mood: null
	});

	let busy = $state(false);
	let error = $state(false);
	let message = $state('');

	function payload(): HooperIndex {
		return { ...values };
	}

	async function run(action: () => Promise<boolean>, successMessage: string) {
		busy = true;
		error = false;
		message = '';

		try {
			const ok = await action();
			error = !ok;
			message = ok ? successMessage : 'Request failed';
		} catch {
			error = true;
			message = 'Invalid values: each measure must be between 1 and 10';
		} finally {
			busy = false;
		}
	}

	const save = () => run(() => createHooperIndex(date, payload()), 'Saved');
	const update = () => run(() => updateHooperIndex(date, payload()), 'Updated');
	const remove = () => run(() => deleteHooperIndex(date), 'Deleted');

	function clear() {
		for (const measure of measures) {
			values[measure] = null;
		}
		message = '';
	}
</script>

<div class="mx-auto flex max-w-md flex-col gap-4 pt-6 pb-10">
	<h1 class="text-xl font-semibold">Hooper index</h1>
	<p class="text-sm italic opacity-60">Temporary test page — not wired to the app.</p>

	<div class="flex flex-col gap-4 rounded-box bg-base-100 p-4 shadow-md">
		<div>
			<label class="label" for="hooper-date">Date</label>
			<input id="hooper-date" type="date" class="input w-full input-sm" bind:value={date} />
		</div>

		{#each measures as measure (measure)}
			<div>
				<label class="label capitalize" for="hooper-{measure}">{measure}</label>
				<input
					id="hooper-{measure}"
					type="number"
					class="input w-full input-sm"
					min="1"
					max="10"
					placeholder="not set"
					bind:value={values[measure]}
				/>
			</div>
		{/each}

		<div class="flex flex-wrap gap-2">
			<button class="btn btn-primary btn-sm" onclick={save} disabled={busy}>Save (POST)</button>
			<button class="btn btn-sm" onclick={update} disabled={busy}>Update (PATCH)</button>
			<button class="btn btn-error btn-sm" onclick={remove} disabled={busy}>Delete</button>
			<button class="btn btn-ghost btn-sm" onclick={clear} disabled={busy}>Clear</button>
		</div>

		{#if busy}
			<span class="loading loading-sm loading-spinner"></span>
		{:else if message}
			<p class={error ? 'text-error' : 'text-success'}>{message}</p>
		{/if}

		<pre class="overflow-x-auto rounded-box bg-base-200 p-2 text-xs">{JSON.stringify(
				{ date, ...values },
				null,
				2
			)}</pre>
	</div>
</div>
