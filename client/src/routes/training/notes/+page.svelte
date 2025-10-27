<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { deleteTrainingNote, updateTrainingNote } from '$lib/api/training';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let notes = $derived(data.notes.toSorted((a, b) => (a.created_at > b.created_at ? -1 : 1)));

	let editingNoteId = $state<string | null>(null);
	let editContent = $state('');
	let deleteConfirmNoteId = $state<string | null>(null);

	const startEdit = (noteId: string, content: string) => {
		editingNoteId = noteId;
		editContent = content;
	};

	const cancelEdit = () => {
		editingNoteId = null;
		editContent = '';
	};

	const saveEdit = async (noteId: string) => {
		if (editContent.trim() === '') return;

		const success = await updateTrainingNote(noteId, editContent.trim());
		if (success) {
			editingNoteId = null;
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

	const deleteNote = async (noteId: string) => {
		const success = await deleteTrainingNote(noteId);
		if (success) {
			deleteConfirmNoteId = null;
			invalidate('app:training-notes');
		}
	};
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box bg-base-100 shadow-md">
		<div class="p-2 px-4 text-sm tracking-wide italic opacity-60">
			Training notes are personal thoughts, observations, and insights about your training journey.
		</div>
		<div>
			{#each notes as note}
				<div class="note-item border-b border-base-200 p-4">
					<div class="mb-2 flex items-center justify-between">
						<div class="text-xs font-light opacity-70">
							{dayjs(note.created_at).format('MMM D, YYYY • HH:mm')}
						</div>
						{#if editingNoteId !== note.id && deleteConfirmNoteId !== note.id}
							<div class="flex gap-2">
								<button
									class="btn btn-ghost btn-xs"
									onclick={() => startEdit(note.id, note.content)}
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
							<textarea class="textarea-bordered textarea w-full" rows="6" bind:value={editContent}
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
					{:else if deleteConfirmNoteId === note.id}
						<div class="alert alert-warning">
							<div class="flex w-full flex-col gap-2">
								<p class="font-semibold">Are you sure you want to delete this note?</p>
								<p class="text-sm">This action cannot be undone.</p>
								<div class="mt-2 flex justify-end gap-2">
									<button class="btn btn-ghost btn-sm" onclick={cancelDelete}>Cancel</button>
									<button class="btn btn-sm btn-error" onclick={() => deleteNote(note.id)}>
										Delete
									</button>
								</div>
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

<style>
	.note-item:hover {
		background: oklch(var(--b2));
	}

	.note-item:last-child {
		border-bottom: none;
	}
</style>
