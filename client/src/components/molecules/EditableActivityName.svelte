<script lang="ts">
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
	<div class="flex flex-col gap-2">
		<div class="text-sm font-medium">Activity Name</div>
		<!-- svelte-ignore a11y_autofocus -->
		<input
			type="text"
			class="input-bordered input input-sm"
			autofocus
			bind:value={editingValue}
			placeholder="Leave empty to use sport name"
		/>
		<div class="flex gap-2">
			<button class="btn btn-sm btn-primary" onclick={handleSave}>💾 Save</button>
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
		</div>
	</div>
{:else}
	<div>
		<span>
			{displayName}
		</span>
		<button class="btn pl-0 opacity-75 btn-ghost btn-xs" onclick={startEditing}> ✏️ </button>
	</div>
{/if}
