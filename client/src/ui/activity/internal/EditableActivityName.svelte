<script lang="ts">
	import EditButton from '$ui/shared/EditButton.svelte';
	import SaveButton from '$ui/shared/SaveButton.svelte';
	import { X } from '@lucide/svelte';

	let {
		name: initialName,
		editCallback
	}: { name: string; editCallback: (newName: string) => Promise<void> } = $props();

	let editMode = $state(false);
	let editingValue = $state('');

	let displayName = $derived(initialName);

	const handleSave = () => {
		editMode = false;
		const trimmedName = editingValue.trim();
		editCallback(trimmedName);
	};

	const handleCancel = () => {
		editMode = false;
	};

	const startEditing = () => {
		editingValue = displayName;
		editMode = true;
	};
</script>

{#if editMode}
	<div class="join mt-3">
		<!-- svelte-ignore a11y_autofocus -->
		<input
			type="text"
			class="input-bordered input join-item input-sm"
			autofocus
			bind:value={editingValue}
			placeholder="Leave empty to use sport name"
			onkeydown={(e) => {
				if (e.key === 'Enter') {
					e.preventDefault();
					handleSave();
				}
			}}
		/>
		<SaveButton callback={handleSave} class="join-item" />
		<button class="btn join-item btn-sm" onclick={handleCancel}>
			<X class="size-4" />
		</button>
	</div>
{:else}
	<div class="flex flex-row items-center gap-0.5">
		<span class="pr-0.5">
			{displayName}
			<EditButton callback={startEditing} />
		</span>
	</div>
{/if}
