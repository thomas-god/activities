<script lang="ts">
	import { onMount, type Snippet } from 'svelte';

	let { children, name }: { children: Snippet; name: string } = $props();

	let container: HTMLDivElement;
	let visible = $state(false);
	let hasBeenVisible = $state(false);

	$effect(() => {
		if (visible) hasBeenVisible = true;
	});

	onMount(() => {
		const io = new IntersectionObserver(
			([entry]) => {
				visible = entry.isIntersecting;
			},
			{ threshold: 0 }
		);
		io.observe(container);
		return () => io.disconnect();
	});
</script>

<div bind:this={container}>
	{#if hasBeenVisible}
		<div style:display={visible ? 'block' : 'none'}>
			{@render children()}
		</div>
	{/if}
</div>
