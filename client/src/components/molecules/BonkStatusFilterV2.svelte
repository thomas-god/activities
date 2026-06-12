<script lang="ts">
	import { BONK_STATUS_VALUES, type BonkStatus } from '$lib/nutrition';
	import { isSome, type Option, none, some } from '$lib/Options';

	let {
		bonkStatus = $bindable()
	}: {
		bonkStatus: Option<BonkStatus>;
	} = $props();
	let editing = $state(false);
</script>

{#if isSome(bonkStatus)}
	<div class="flex flex-col gap-1 rounded border border-primary/40 p-1.5 shadow">
		<div class="flex flex-row items-center justify-between">
			<div class="flex-1 text-wrap break-words">
				Bonk status: {bonkStatus.value === 'none' ? 'No bonk' : 'Bonked'}
			</div>
			<div class="join shrink-0">
				<button class="btn join-item btn-xs" onclick={() => (editing = !editing)}>
					<img src="/icons/edit.svg" alt="Pen editing icon" class="inline h-5 w-5" />
				</button>
				<button class="btn join-item btn-xs" onclick={() => (bonkStatus = none())}>
					<img src="/icons/delete.svg" alt="Bin delete icon" class="inline h-5 w-5" />
				</button>
			</div>
		</div>
		{#if editing}
			<div class="divider my-0.5 px-2"></div>
			<div class="flex gap-4">
				{#each BONK_STATUS_VALUES as status (status)}
					<label class="label cursor-pointer justify-start gap-2">
						<input
							type="radio"
							name="bonk-status"
							class="radio radio-xs"
							value={status}
							checked={bonkStatus.value === status}
							onchange={() => {
								bonkStatus = some(status);
							}}
						/>
						<span class="label-text text-xs capitalize"
							>{status === 'none' ? 'No bonk' : 'Bonked'}</span
						>
					</label>
				{/each}
			</div>
		{/if}
	</div>
{/if}
