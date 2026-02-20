<script lang="ts">
	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button class="btn btn-ghost btn-xs" onclick={callback}
	><img src="/icons/edit.svg" class="h-4 w-4" alt="Edit icon" />{text}</button
>
