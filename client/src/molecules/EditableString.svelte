<script lang="ts">
	let {
		content: initialContent,
		editCallback
	}: { content: string; editCallback: (newContent: string) => Promise<void> } = $props();

	let content = $state(initialContent);
	let editMode = $state(false);
</script>

{#if editMode}
	<div class="join">
		<input class="input join-item" autofocus bind:value={content} />
		<button
			class="btn join-item rounded-r-full"
			onclick={() => {
				editMode = false;
				editCallback(content);
			}}>save</button
		>
	</div>
{:else}
	<div class="join">
		<div class="join-item">{content}</div>
		<button class="btn join-item rounded-r-full" onclick={() => (editMode = true)}>edit</button>
	</div>
{/if}
