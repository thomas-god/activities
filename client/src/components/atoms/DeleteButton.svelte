<script lang="ts">
	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button onclick={callback} class="btn text-error btn-ghost">
	<img src="/icons/delete.svg" class="h-4 w-4" alt="" />
	<span>{text}</span>
</button>
