<script lang="ts">
	import { deleteTrainingNote, updateTrainingNote } from '$lib/api/training';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import TrainingNoteListItem from '../../../organisms/TrainingNoteListItem.svelte';

	let { data }: PageProps = $props();

	let notes = $derived(data.notes.toSorted((a, b) => (a.date > b.date ? -1 : 1)));

	const saveNote = async (noteId: string, content: string, date: string) => {
		const success = await updateTrainingNote(noteId, content, date);
		if (success) {
			invalidate('app:training-notes');
		}
	};

	const deleteNote = async (noteId: string) => {
		const success = await deleteTrainingNote(noteId);
		if (success) {
			invalidate('app:training-notes');
		}
	};
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box bg-base-100 shadow-md">
		<div class="p-4 text-sm tracking-wide italic opacity-60">
			Training notes are personal observations, insights and decisions about your training.
		</div>
		<div>
			{#each notes as note}
				<div class="px-4 py-2">
					<TrainingNoteListItem
						{note}
						onSave={(content, date) => saveNote(note.id, content, date)}
						onDelete={() => deleteNote(note.id)}
					/>
				</div>
			{:else}
				<div class="italic text-sm text-center tracking-wide opacity-60 p-8">
					No training notes yet. Click "+ New note" to create your first one.
				</div>
			{/each}
		</div>
	</div>
</div>
