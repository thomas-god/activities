<script lang="ts">
	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button onclick={callback} class="btn btn-ghost text-error">
	<img src="/icons/delete.svg" class="h-4 w-4" alt="Delete icon" />
	<span>{text}</span>
</button>
