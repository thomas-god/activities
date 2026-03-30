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
	<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
		<!-- row 1: color swatch + label + +/- controls (+ reset) -->
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<span class="inline-block h-0.5 w-5 shrink-0 rounded-full" style="background-color:{color}"
			></span>
			<span class="min-w-0 flex-1 truncate opacity-75 sm:w-28 sm:flex-none">{label}</span>
		</div>
		<div class="flex shrink-0 items-center">
			<button
				class="btn px-1 btn-ghost btn-xs"
				aria-label="Decrease offset by 10 seconds"
				onclick={() => offsets.set(id, Math.max(-3600, (offsets.get(id) ?? 0) - 10))}>−</button
			>
			<span class="w-14 text-center font-mono text-xs opacity-60"
				>{fmtOffset(offsets.get(id) ?? 0)}</span
			>
			<button
				class="btn px-1 btn-ghost btn-xs"
				aria-label="Increase offset by 10 seconds"
				onclick={() => offsets.set(id, Math.min(3600, (offsets.get(id) ?? 0) + 10))}>+</button
			>
		</div>
		<button
			class="btn px-1 opacity-50 btn-ghost btn-xs"
			class:invisible={(offsets.get(id) ?? 0) === 0}
			onclick={() => offsets.set(id, 0)}
			aria-label="Reset offset">✕</button
		>
		<!-- row 2: slider, full width on small screens, inline on sm+ -->
		<input
			type="range"
			min="-3600"
			max="3600"
			step="10"
			class="range w-full range-xs sm:w-auto sm:flex-1"
			value={offsets.get(id) ?? 0}
			oninput={(e) => {
				offsets.set(id, Number((e.target as HTMLInputElement).value));
			}}
		/>
	</div>
{/each}
