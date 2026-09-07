<script lang="ts">
	import { Save } from '@lucide/svelte';

	let {
		callback,
		text = '',
		class: extraClass = ''
	}: { callback: () => void; text?: string; class?: string } = $props();

	let actualClass = $derived(extraClass + ' btn btn-accent btn-sm');
</script>

<button class={actualClass} onclick={callback}>
	<Save class="size-4 stroke-1" />
	{text}
</button>
