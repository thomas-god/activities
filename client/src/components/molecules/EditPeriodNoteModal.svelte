<script lang="ts">
	interface Props {
		isOpen: boolean;
		currentNote: string | null;
		onConfirm: (note: string) => Promise<void>;
	}

	let { isOpen = $bindable(false), currentNote, onConfirm }: Props = $props();

	let editedNote = $state('');
	let isUpdating = $state(false);

	$effect(() => {
		if (isOpen) {
			editedNote = currentNote ?? '';
		}
	});

	const handleConfirm = async () => {
		isUpdating = true;
		try {
			await onConfirm(editedNote.trim());
			isOpen = false;
		} finally {
			isUpdating = false;
		}
	};
</script>

{#if isOpen}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">
				{currentNote ? 'Edit' : 'Add'} training period description
			</h3>
			<div class="py-4">
				<label class="floating-label">
					<textarea
						bind:value={editedNote}
						placeholder="Add a description of this training period..."
						class="textarea w-full"
						rows="6"
						disabled={isUpdating}
					></textarea>
					<span>Description</span>
				</label>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={() => (isOpen = false)} disabled={isUpdating}>Cancel</button>
				<button class="btn btn-primary" onclick={handleConfirm} disabled={isUpdating}>
					{#if isUpdating}
						<span class="loading loading-sm loading-spinner"></span>
						Saving...
					{:else}
						Save
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={() => (isOpen = false)}>close</button>
		</form>
	</dialog>
{/if}
