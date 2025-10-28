<script lang="ts">
	import { deleteTrainingNote, updateTrainingNote } from '$lib/api/training';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import TrainingNoteListItem from '../../../organisms/TrainingNoteListItem.svelte';

	let { data }: PageProps = $props();

	let notes = $derived(data.notes.toSorted((a, b) => (a.date > b.date ? -1 : 1)));

	let deleteConfirmNoteId = $state<string | null>(null);
	let isDeleting = $state(false);

	const saveNote = async (noteId: string, content: string, date: string) => {
		const success = await updateTrainingNote(noteId, content, date);
		if (success) {
			invalidate('app:training-notes');
		}
	};

	const confirmDelete = (noteId: string) => {
		deleteConfirmNoteId = noteId;
	};

	const cancelDelete = () => {
		deleteConfirmNoteId = null;
	};

	const deleteNote = async () => {
		if (!deleteConfirmNoteId) return;

		isDeleting = true;
		const success = await deleteTrainingNote(deleteConfirmNoteId);
		if (success) {
			deleteConfirmNoteId = null;
			invalidate('app:training-notes');
		}
		isDeleting = false;
	};

	// Find the note content for the delete modal
	const noteToDelete = $derived(notes.find((n) => n.id === deleteConfirmNoteId));
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box bg-base-100 shadow-md">
		<div class="p-4 text-sm tracking-wide italic opacity-60">
			Training notes are personal observations, insights and decisions about your training.
		</div>
		<div>
			{#each notes as note}
				<TrainingNoteListItem
					{note}
					onSave={(content, date) => saveNote(note.id, content, date)}
					onDelete={() => confirmDelete(note.id)}
				/>
			{:else}
				<div class="italic text-sm text-center tracking-wide opacity-60 p-8">
					No training notes yet. Click "+ New note" to create your first one.
				</div>
			{/each}
		</div>
	</div>
</div>

<!-- Delete confirmation modal -->
{#if deleteConfirmNoteId}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Delete Training Note</h3>
			<p class="py-4">
				Are you sure you want to delete this note?
				{#if noteToDelete}
					<br />
					<span class="mt-2 block text-sm italic opacity-70">
						<span class="line-clamp-3">
							{noteToDelete.content.slice(0, 75)}{noteToDelete.content.length > 75 ? '...' : ''}
						</span>
					</span>
				{/if}
				<br />
				<strong>This action cannot be undone.</strong>
			</p>
			<div class="modal-action">
				<button class="btn" onclick={cancelDelete} disabled={isDeleting}> Cancel </button>
				<button class="btn btn-error" onclick={deleteNote} disabled={isDeleting}>
					{#if isDeleting}
						<span class="loading loading-sm loading-spinner"></span>
						Deleting...
					{:else}
						Delete
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={cancelDelete}>close</button>
		</form>
	</dialog>
{/if}
