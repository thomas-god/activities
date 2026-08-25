<script lang="ts">
	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button
	class="btn mr-auto btn-ghost px-1 text-sm font-medium btn-xs"
	style="color: color-mix(in oklab, var(--color-primary) 75%, var(--color-base-content) 25%);"
	onclick={callback}><img src="/icons/plus.svg" class="h-3.5 w-3.5" alt="Add icon" />{text}</button
>
