<script lang="ts">
	import { BONK_STATUS_VALUES, type BonkStatus } from '$lib/nutrition';

	let {
		enabled = $bindable(false),
		selectedBonkStatus = $bindable(null)
	}: {
		enabled?: boolean;
		selectedBonkStatus?: BonkStatus | null;
	} = $props();
</script>

<div class="mb-2 font-semibold">
	<span class="pr-2"> Filter activities by bonk status </span>
	<input type="checkbox" bind:checked={enabled} class="toggle toggle-sm" />
</div>
{#if enabled}
	<div class="flex gap-4">
		{#each BONK_STATUS_VALUES as status (status)}
			<label class="label cursor-pointer justify-start gap-2">
				<input
					type="radio"
					name="bonk-status"
					class="radio radio-sm"
					value={status}
					checked={selectedBonkStatus === status}
					onchange={() => {
						selectedBonkStatus = status;
					}}
				/>
				<span class="label-text text-xs capitalize">{status === 'none' ? 'No bonk' : 'Bonked'}</span
				>
			</label>
		{/each}
	</div>
{/if}
