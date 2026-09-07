<script lang="ts">
	import { Pencil } from '@lucide/svelte';

	let {
		callback,
		text = '',
		size = 'small',
		class: extraClass = ''
	}: { callback: () => void; text?: string; size?: 'small' | 'normal'; class?: string } = $props();

	let actualClass = $derived(
		`btn btn-circle btn-ghost ${size === 'small' ? 'btn-xs' : ''} ${extraClass}`
	);
</script>

<button class={actualClass} onclick={callback}> <Pencil class="size-3.5 " />{text}</button>
