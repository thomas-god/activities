<script lang="ts">
	import { formatDuration } from '$lib/duration';
	import type { SvelteMap } from 'svelte/reactivity';

	let {
		activities,
		offsets = $bindable()
	}: {
		activities: { color: string; label: string; id: string }[];
		offsets: SvelteMap<string, number>;
	} = $props();

	const fmtOffset = (s: number): string => {
		if (s === 0) return '0:00';
		return (s < 0 ? '-' : '+') + formatDuration(Math.abs(s));
	};
</script>

{#each activities as { color, label, id }}
	<div class="flex items-center gap-2">
		<span class="inline-block h-0.5 w-5 shrink-0 rounded-full" style="background-color:{color}"
		></span>
		<span class="w-28 shrink-0 truncate opacity-75">{label}</span>
		<input
			type="range"
			min="-3600"
			max="3600"
			step="10"
			class="range flex-1 range-xs"
			value={offsets.get(id) ?? 0}
			oninput={(e) => {
				offsets.set(id, Number((e.target as HTMLInputElement).value));
			}}
		/>
		<span class="w-14 shrink-0 text-right font-mono text-xs opacity-60"
			>{fmtOffset(offsets.get(id) ?? 0)}</span
		>
		{#if (offsets.get(id) ?? 0) !== 0}
			<button
				class="btn px-1 opacity-50 btn-ghost btn-xs"
				onclick={() => {
					offsets.set(id, 0);
				}}>✕</button
			>
		{/if}
	</div>
{/each}
