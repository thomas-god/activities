<script lang="ts">
	import { Save } from '@lucide/svelte';

	let { callback, text = '' }: { callback: () => void; text?: string } = $props();
</script>

<button class="btn btn-primary btn-sm" onclick={callback}>
	<Save class="size-4" />
	{text}
</button>
