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
			}}>💾</button
		>
	</div>
{:else}
	<div class="join items-center">
		<span class="join-item">{content}</span>
		<button
			class="btn join-item border-0 bg-transparent opacity-75 btn-sm"
			onclick={() => (editMode = true)}
			>✏️
		</button>
	</div>
{/if}
