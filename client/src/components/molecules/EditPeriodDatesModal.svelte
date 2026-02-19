<script lang="ts">
	interface Props {
		isOpen: boolean;
		currentStart: string;
		currentEnd: string | null;
		onConfirm: (start: string, end?: string) => Promise<void>;
	}

	let { isOpen = $bindable(false), currentStart, currentEnd, onConfirm }: Props = $props();

	let editedStart = $state('');
	let editedEnd = $state('');
	let isUpdating = $state(false);

	$effect(() => {
		if (isOpen) {
			editedStart = currentStart;
			editedEnd = currentEnd ?? '';
		}
	});

	const handleConfirm = async () => {
		if (!editedStart) {
			alert('Start date is required');
			return;
		}
		if (editedEnd && editedEnd < editedStart) {
			alert('End date must be after start date');
			return;
		}
		isUpdating = true;
		try {
			await onConfirm(editedStart, editedEnd || undefined);
			isOpen = false;
		} finally {
			isUpdating = false;
		}
	};
</script>

{#if isOpen}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Period Dates</h3>
			<div class="space-y-4 py-4">
				<label class="input">
					<span class="label">Start Date</span>
					<input
						type="date"
						bind:value={editedStart}
						class="w-full text-right"
						disabled={isUpdating}
						required
					/>
				</label>
				<label class="input">
					<span class="label">End Date (optional)</span>
					<input
						type="date"
						bind:value={editedEnd}
						class="w-full text-right"
						disabled={isUpdating}
						min={editedStart}
					/>
				</label>
				<p class="text-xs opacity-70">Leave end date empty for an ongoing period</p>
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
