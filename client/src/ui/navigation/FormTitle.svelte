<script lang="ts">
	import { isSome, none, type Option } from '$lib/Options';
	import { ArrowLeft, ArrowRight } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		content,
		next = none(),
		previous = none()
	}: {
		content: Snippet;
		next?: Option<() => void>;
		previous?: Option<() => void>;
	} = $props();
</script>

<legend
	class="fieldset-legend mt-1 mb-3 flex w-full flex-row justify-center gap-2 overflow-scroll text-base xs:mb-1"
>
	{#if isSome(previous)}
		<button onclick={previous.value} class="btn btn-ghost btn-xs">
			<ArrowLeft class="size-4" />
		</button>
	{/if}
	<span class="flex w-48 flex-row items-center justify-center gap-2">
		{@render content()}
	</span>
	{#if isSome(next)}
		<button onclick={next.value} class="btn btn-ghost btn-xs">
			<ArrowRight class="size-4" />
		</button>
	{/if}
</legend>
