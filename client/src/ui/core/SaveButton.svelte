<script lang="ts">
	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button class="btn btn-primary btn-sm" onclick={callback}
	><img src="/icons/save.svg" class="h-4 w-4" alt="Save icon" />{text}</button
>
