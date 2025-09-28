<script lang="ts">
	let { dates = $bindable() }: { dates: { start: string | null; end: string | null } } = $props();

	let start = $derived(dates.start);
	let end = $derived(dates.end);

	const callback = () => {
		dates = { start, end };
	};
</script>

<div class="flex justify-start gap-2 p-2 sm:px-4">
	<label class="label">
		From
		<input type="date" class="input" bind:value={start} max={end} oninput={callback} />
	</label>
	<label class="label">
		to
		<input type="date" class="input" bind:value={end} min={start} oninput={callback} />
	</label>
</div>
