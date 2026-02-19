<script lang="ts">
	interface Props {
		isOpen: boolean;
		currentName: string;
		onConfirm: (name: string) => Promise<void>;
	}

	let { isOpen = $bindable(false), currentName, onConfirm }: Props = $props();

	let editedName = $state('');
	let isUpdating = $state(false);

	$effect(() => {
		if (isOpen) {
			editedName = currentName;
		}
	});

	const handleConfirm = async () => {
		if (!editedName.trim()) {
			alert('Name cannot be empty');
			return;
		}
		isUpdating = true;
		try {
			await onConfirm(editedName.trim());
			isOpen = false;
		} finally {
			isUpdating = false;
		}
	};
</script>

{#if isOpen}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Period Name</h3>
			<div class="py-4">
				<label class="input">
					<span class="label">Name</span>
					<input
						type="text"
						bind:value={editedName}
						placeholder="Enter period name"
						class="w-full"
						disabled={isUpdating}
					/>
				</label>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={() => (isOpen = false)} disabled={isUpdating}>Cancel</button>
				<button class="btn btn-primary" onclick={handleConfirm} disabled={isUpdating}>
					{#if isUpdating}
						<span class="loading loading-sm loading-spinner"></span>
						Updating...
					{:else}
						Update
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={() => (isOpen = false)}>close</button>
		</form>
	</dialog>
{/if}
