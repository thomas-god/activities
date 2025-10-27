<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { deleteTrainingNote, updateTrainingNote } from '$lib/api/training';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let notes = $derived(data.notes.toSorted((a, b) => (a.created_at > b.created_at ? -1 : 1)));

	let editingNoteId = $state<string | null>(null);
	let editTitle = $state('');
	let editContent = $state('');
	let deleteConfirmNoteId = $state<string | null>(null);
	let isDeleting = $state(false);

	const startEdit = (noteId: string, title: string | null, content: string) => {
		editingNoteId = noteId;
		editTitle = title || '';
		editContent = content;
	};

	const cancelEdit = () => {
		editingNoteId = null;
		editTitle = '';
		editContent = '';
	};

	const saveEdit = async (noteId: string) => {
		if (editContent.trim() === '') return;

		const success = await updateTrainingNote(
			noteId,
			editTitle.trim() || undefined,
			editContent.trim()
		);
		if (success) {
			editingNoteId = null;
			editTitle = '';
			editContent = '';
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
		<div class="p-2 px-4 text-sm tracking-wide italic opacity-60">
			Training notes are personal observations, insights and decisions about your training.
		</div>
		<div>
			{#each notes as note}
				<div class="note-item border-b border-base-200 p-4">
					{#if editingNoteId !== note.id}
						{#if note.title}
							<h3 class="mb-2 text-lg font-semibold">{note.title}</h3>
						{/if}
					{/if}
					<div class="mb-2 flex items-center justify-between">
						<div class="text-xs font-light opacity-70">
							{dayjs(note.created_at).format('MMM D, YYYY • HH:mm')}
						</div>
						{#if editingNoteId !== note.id}
							<div class="flex gap-2">
								<button
									class="btn btn-ghost btn-xs"
									onclick={() => startEdit(note.id, note.title, note.content)}
								>
									✏️ Edit
								</button>
								<button class="btn btn-ghost btn-xs" onclick={() => confirmDelete(note.id)}>
									🗑️ Delete
								</button>
							</div>
						{/if}
					</div>

					{#if editingNoteId === note.id}
						<div class="flex flex-col gap-2">
							<input
								type="text"
								class="input-bordered input w-full"
								placeholder="Title (optional)"
								bind:value={editTitle}
							/>
							<textarea
								class="textarea-bordered textarea w-full"
								rows="6"
								placeholder="Content"
								bind:value={editContent}
							></textarea>
							<div class="flex justify-end gap-2">
								<button class="btn btn-ghost btn-sm" onclick={cancelEdit}>Cancel</button>
								<button
									class="btn btn-sm btn-primary"
									onclick={() => saveEdit(note.id)}
									disabled={editContent.trim() === ''}
								>
									Save
								</button>
							</div>
						</div>
					{:else}
						<div class="text-sm whitespace-pre-wrap">{note.content}</div>
					{/if}
				</div>
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
						{#if noteToDelete.title}
							<strong>"{noteToDelete.title}"</strong>
							<br />
						{/if}
						<span class="line-clamp-3">
							{noteToDelete.content.slice(0, 100)}{noteToDelete.content.length > 100 ? '...' : ''}
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

<style>
	.note-item:hover {
		background: oklch(var(--b2));
	}

	.note-item:last-child {
		border-bottom: none;
	}
</style>
