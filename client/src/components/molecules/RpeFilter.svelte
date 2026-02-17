<script lang="ts">
	import { RPE_VALUES } from '$lib/rpe';

	let {
		enabled = $bindable(false),
		selectedRpes = $bindable([])
	}: {
		enabled?: boolean;
		selectedRpes?: number[];
	} = $props();
</script>

<div class="mb-2 font-semibold">
	<span class="pr-2"> Filter activities by RPE </span>
	<input type="checkbox" bind:checked={enabled} class="toggle toggle-sm" />
</div>
{#if enabled}
	<div class="grid grid-cols-5 gap-2">
		{#each RPE_VALUES as rpe (rpe)}
			<label class="label cursor-pointer justify-start gap-2">
				<input
					type="checkbox"
					class="checkbox checkbox-sm"
					value={rpe}
					checked={selectedRpes.includes(rpe)}
					onchange={(e) => {
						if (e.currentTarget.checked) {
							selectedRpes = [...selectedRpes, rpe];
						} else {
							selectedRpes = selectedRpes.filter((r) => r !== rpe);
						}
					}}
				/>
				<span class="label-text text-xs">{rpe}/10</span>
			</label>
		{/each}
	</div>
{/if}
