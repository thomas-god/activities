<script lang="ts">
	import { isSome, type Option, none, some } from '$lib/Options';
	import { RPE_VALUES, type RPEValue } from '$lib/rpe';
	import { Pencil, Trash2 } from '@lucide/svelte';

	let {
		rpes = $bindable()
	}: {
		rpes: Option<RPEValue[]>;
	} = $props();
	let editing = $state(false);

	const addRpe = (rpe: RPEValue) => {
		if (isSome(rpes)) {
			rpes = some([...rpes.value, rpe]);
		}
	};
	const removeRpe = (rpe: RPEValue) => {
		if (isSome(rpes)) {
			rpes = some(rpes.value.filter((r) => r !== rpe));
		}
	};
</script>

{#if isSome(rpes)}
	<div class="flex flex-col gap-1 rounded border border-black/30 p-1.5 shadow">
		<div class="flex flex-row items-center justify-between">
			<div class="flex-1 text-wrap wrap-break-word">
				RPEs: {rpes.value.toSorted().join(', ')}
			</div>
			<div class="join shrink-0">
				<button class="btn join-item btn-xs" onclick={() => (editing = !editing)}>
					<Pencil class="size-4" />
				</button>
				<button class="btn join-item btn-xs" onclick={() => (rpes = none())}>
					<Trash2 class="size-4" />
				</button>
			</div>
		</div>
		{#if editing}
			<div class="divider my-0.5 px-2"></div>
			<div class="grid grid-cols-5 gap-2 px-1">
				{#each RPE_VALUES as rpe (rpe)}
					<label class="label cursor-pointer justify-start gap-2">
						<input
							type="checkbox"
							class="checkbox checkbox-xs"
							value={rpe}
							checked={rpes.value.includes(rpe)}
							onchange={(e) => {
								if (e.currentTarget.checked) {
									addRpe(rpe);
								} else {
									removeRpe(rpe);
								}
							}}
						/>
						<span class="label-text text-xs">{rpe}</span>
					</label>
				{/each}
			</div>
		{/if}
	</div>
{/if}
