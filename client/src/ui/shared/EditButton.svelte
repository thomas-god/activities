<script lang="ts">
	import { Pencil } from '@lucide/svelte';

	let {
		callback,
		text = '',
		size = 'small'
	}: { callback: () => void; text?: string; size?: 'small' | 'normal' } = $props();
</script>

<button class={`btn btn-ghost ${size === 'small' ? 'btn-xs' : ''}`} onclick={callback}>
	<Pencil class="size-4" />{text}</button
>
