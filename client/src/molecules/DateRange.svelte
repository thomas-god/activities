<script lang="ts">
	let { dates = $bindable() }: { dates: { start: string | null; end: string | null } } = $props();

	let start = $state(dates.start);
	let end = $state(dates.end);

	const callback = () => {
		dates = { start, end };
	};
</script>

<div class="flex justify-around p-2">
	<label class="label">
		start
		<input type="date" class="input" bind:value={start} max={end} oninput={callback} />
	</label>
	<label class="label">
		end
		<input type="date" class="input" bind:value={end} min={start} oninput={callback} />
	</label>
</div>
