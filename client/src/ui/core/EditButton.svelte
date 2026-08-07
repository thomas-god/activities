<script lang="ts">
	let {
		callback,
		text = '',
		size = 'small'
	}: { callback: () => void; text?: string; size?: 'small' | 'normal' } = $props();
</script>

<button class={`btn btn-ghost ${size === 'small' ? 'btn-xs' : ''}`} onclick={callback}
	><img src="/icons/edit.svg" class="h-4 w-4" alt="Edit icon" />{text}</button
>
