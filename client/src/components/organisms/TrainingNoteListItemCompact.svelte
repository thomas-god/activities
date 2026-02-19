<script lang="ts">
	import { dayjs, formatRelativeDuration } from '$lib/duration';
	import type { TrainingNote } from '$lib/api/training';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';

	let {
		note,
		onEdit,
		onDelete
	}: {
		note: TrainingNote;
		onEdit?: (content: string, date: string) => void;
		onDelete?: () => void;
	} = $props();

	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let isSaving = $state(false);
	let editContent = $state('');
	let editDate = $state('');

	const startEdit = (e: Event) => {
		e.stopPropagation();
		showEditModal = true;
		editContent = note.content;
		editDate = note.date;
	};

	const cancelEdit = () => {
		showEditModal = false;
		editContent = '';
		editDate = '';
	};

	const saveEdit = async () => {
		if (editContent.trim() === '') return;
		isSaving = true;
		await onEdit?.(editContent.trim(), editDate);
		isSaving = false;
		showEditModal = false;
		editContent = '';
		editDate = '';
	};

	const confirmDelete = (e: Event) => {
		e.stopPropagation();
		showDeleteModal = true;
	};

	const handleDelete = async () => {
		await onDelete?.();
	};
</script>

<div class="item_container @container flex w-full flex-1 flex-col items-stretch p-2 text-left">
	<!-- Icon + Title and date -->
	<div class="flex w-full flex-row justify-start">
		<div class="icon">💭</div>
		<div class="flex flex-1 flex-col">
			<div class=" font-semibold">Note</div>
			<div class="flex flex-row items-center gap-2">
				<div class="text-xs font-light">
					{formatRelativeDuration(dayjs(note.date), dayjs())} · {dayjs(note.date).format(
						'MMM D, YYYY'
					)}
				</div>
				<div class="dropdown dropdown-end">
					<div tabindex="0" role="button" class="btn btn-square btn-ghost btn-xs">⋮</div>
					<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
					<ul
						tabindex="0"
						class="dropdown-content menu z-[1] w-32 rounded-box bg-base-100 p-2 shadow"
					>
						<li>
							<button onclick={startEdit}>
								<span>✏️</span>
								<span>Edit</span>
							</button>
						</li>
						<li>
							<button onclick={confirmDelete} class="text-error">
								<span>🗑️</span>
								<span>Delete</span>
							</button>
						</li>
					</ul>
				</div>
			</div>
		</div>
	</div>
	<div
		class="mx-1 my-1 box-border flex flex-row gap-1 bg-orange-200/10 py-2 pl-2 text-sm whitespace-pre-wrap text-gray-600 italic"
	>
		<div>📝</div>
		<div>
			{note.content}
		</div>
	</div>
</div>

<!-- Edit modal -->
{#if showEditModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Note</h3>
			<div class="flex flex-col gap-4 py-4">
				<label class="floating-label">
					<input type="date" class="input" bind:value={editDate} disabled={isSaving} />
					<span>Date</span>
				</label>

				<textarea
					class="textarea-bordered textarea w-full"
					rows="8"
					placeholder="Content"
					bind:value={editContent}
					disabled={isSaving}
				></textarea>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={cancelEdit} disabled={isSaving}> Cancel </button>
				<button
					class="btn btn-primary"
					onclick={saveEdit}
					disabled={editContent.trim() === '' || isSaving}
				>
					{#if isSaving}
						<span class="loading loading-sm loading-spinner"></span>
						Saving...
					{:else}
						Save
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={cancelEdit}>close</button>
		</form>
	</dialog>
{/if}

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Training Note"
	description="Are you sure you want to delete this note?"
	itemPreview={note.content.slice(0, 75) + (note.content.length > 75 ? '...' : '')}
	onConfirm={handleDelete}
/>

<style>
	.item_container {
		padding-block: calc(var(--spacing) * 1);
		padding-right: calc(var(--spacing) * 1);
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 0px;
		border-color: var(--color-orange-300);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
		background-color: color-mix(in oklab, var(--color-orange-200) 60%, transparent);
	}
</style>
