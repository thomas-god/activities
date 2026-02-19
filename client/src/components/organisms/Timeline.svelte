<script lang="ts">
	import type { Activity, ActivityList, TrainingNote, TrainingNotesList } from '$lib/api';
	import ActivitiesListItem from './ActivitiesListItem.svelte';
	import TrainingNoteListItemCompact from './TrainingNoteListItemCompact.svelte';

	let {
		activities,
		notes,
		selectedActivityId,
		selectActivityCallback,
		saveNoteCallback,
		deleteNoteCallback
	}: {
		activities: ActivityList;
		notes: TrainingNotesList;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		saveNoteCallback: (id: string, content: string, date: string) => void;
		deleteNoteCallback: (id: string) => void;
	} = $props();

	// Merge activities and notes, sorted by date (most recent first)
	type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	const timeline = $derived.by((): TimelineItem[] => {
		const items: TimelineItem[] = [
			...activities.map((activity) => ({
				type: 'activity' as const,
				data: activity,
				date: activity.start_time
			})),
			...notes.map((note) => ({
				type: 'note' as const,
				data: note,
				date: note.date
			}))
		];

		return items.sort((a, b) => (a.date > b.date ? -1 : 1));
	});
</script>

{#if timeline.length > 0}
	<div class="flex flex-col gap-0">
		{#each timeline as item}
			{#if item.type === 'activity'}
				<ActivitiesListItem
					activity={item.data}
					showNote={true}
					onClick={() => selectActivityCallback(item.data.id)}
					isSelected={selectedActivityId === item.data.id}
				/>
			{:else}
				<TrainingNoteListItemCompact
					note={item.data}
					onEdit={(content, date) => saveNoteCallback(item.data.id, content, date)}
					onDelete={() => deleteNoteCallback(item.data.id)}
				/>
			{/if}
		{/each}
	</div>
{:else}
	<div class="py-8 text-center text-sm italic opacity-70">
		No activities in this training period yet
	</div>
{/if}
