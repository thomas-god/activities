<script lang="ts">
	import { deleteTrainingNote, updateTrainingNote } from '$lib/api/training';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import TrainingNoteListItemCompact from '$components/organisms/TrainingNoteListItemCompact.svelte';

	let { data }: PageProps = $props();

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
	<div class="rounded-box bg-base-100 pb-3 shadow-md">
		<div class="p-4 text-sm tracking-wide italic opacity-60">
			Training notes are personal observations, insights and decisions about your training.
		</div>
		<div>
			{#await data.notes}
				<div class="flex w-full flex-col items-center p-4 pt-6">
					<div class="loading loading-bars"></div>
				</div>
			{:then notes}
				{#each notes as note}
					<div class="px-2 sm:px-4">
						<TrainingNoteListItemCompact
							{note}
							onEdit={(content, date) => saveNote(note.id, content, date)}
							onDelete={() => deleteNote(note.id)}
						/>
					</div>
				{:else}
					<div class="italic text-sm text-center tracking-wide opacity-60 p-8">
						No training notes yet. Click "+ New note" to create your first one.
					</div>
				{/each}
			{/await}
		</div>
	</div>
</div>
