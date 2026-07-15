<!-- ResponsiveLazySwitch.svelte -->
<script lang="ts">
	import { isSome, none, some, type Option } from '$lib/Options';
	import { onMount, type Snippet } from 'svelte';

	let { breakpoint, compact, full }: { compact: Snippet; full: Snippet; breakpoint: number } =
		$props();

	let containerEl: HTMLDivElement;
	let isCompact: Option<boolean> = $state(none());

	onMount(() => {
		const ro = new ResizeObserver(([entry]) => {
			isCompact = some(entry.contentRect.width < breakpoint);
		});
		ro.observe(containerEl);
		return () => ro.disconnect();
	});
</script>

<div bind:this={containerEl} style="container-type: inline-size;">
	{#if isSome(isCompact)}
		<div style:display={isCompact.value ? 'block' : 'none'} class="transition">
			{@render compact()}
		</div>
		<div style:display={!isCompact.value ? 'block' : 'none'} class="transition">
			{@render full()}
		</div>
	{/if}
</div>
