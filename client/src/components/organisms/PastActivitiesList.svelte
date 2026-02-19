<script lang="ts">
	import { dayjs } from '$lib/duration';
	import type { ActivityList, Activity } from '$lib/api';
	import type { TrainingNote, TrainingNotesList } from '$lib/api/training';
	import ActivitiesListItem from './ActivitiesListItem.svelte';
	import TrainingNoteListItemCompact from './TrainingNoteListItemCompact.svelte';

	type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	let {
		activityList,
		trainingNotes = [],
		moreCallback,
		onNoteSave,
		onNoteDelete,
		onActivityClick,
		selectedActivityId = null
	}: {
		activityList: ActivityList;
		trainingNotes?: TrainingNotesList;
		moreCallback: () => void;
		onNoteSave?: (noteId: string, content: string, date: string) => void;
		onNoteDelete?: (noteId: string) => void;
		onActivityClick?: (activityId: string) => void;
		selectedActivityId?: string | null;
	} = $props();

	let sorted_activities = $derived(
		activityList.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	let groupedItems = $derived.by(() => {
		const now = dayjs();
		const items = {
			thisWeek: [] as TimelineItem[],
			thisMonth: [] as TimelineItem[],
			earlier: [] as TimelineItem[]
		};

		// Add activities to timeline
		for (const activity of sorted_activities) {
			const date = dayjs(activity.start_time);
			const item: TimelineItem = {
				type: 'activity',
				data: activity,
				date: activity.start_time
			};

			if (date > now.startOf('isoWeek')) {
				items.thisWeek.push(item);
			} else if (date > now.startOf('month')) {
				items.thisMonth.push(item);
			} else {
				items.earlier.push(item);
			}
		}

		// Add training notes to timeline
		for (const note of trainingNotes) {
			const date = dayjs(note.date);
			const item: TimelineItem = {
				type: 'note',
				data: note,
				date: note.date
			};

			if (date > now.startOf('isoWeek')) {
				items.thisWeek.push(item);
			} else if (date > now.startOf('month')) {
				items.thisMonth.push(item);
			} else {
				items.earlier.push(item);
			}
		}

		// Sort each group by date (most recent first)
		items.thisWeek.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));
		items.thisMonth.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));
		items.earlier.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));

		return items;
	});

	const containerClass = 'text-base-content/60 my-3 text-xs font-semibold uppercase tracking-wide';
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="flex items-center justify-between pb-2 text-lg font-semibold tracking-wide">
		<span> Recent activities and notes</span>
		<button class="btn btn-link btn-sm" onclick={moreCallback}> view all →</button>
	</div>

	{#if groupedItems.thisWeek.length > 0}
		<p class={containerClass}>This week</p>
		<div class="flex flex-col">
			{#each groupedItems.thisWeek as item}
				{#if item.type === 'activity'}
					<ActivitiesListItem
						activity={item.data}
						showNote={true}
						onClick={() => onActivityClick?.(item.data.id)}
						isSelected={selectedActivityId === item.data.id}
					/>
				{:else}
					<TrainingNoteListItemCompact
						note={item.data}
						onEdit={(content, date) => onNoteSave?.(item.data.id, content, date)}
						onDelete={() => onNoteDelete?.(item.data.id)}
					/>
				{/if}
			{/each}
		</div>
	{/if}

	{#if groupedItems.thisMonth.length > 0}
		<p class={containerClass}>This month</p>
		<div class="flex flex-col">
			{#each groupedItems.thisMonth as item}
				{#if item.type === 'activity'}
					<ActivitiesListItem
						activity={item.data}
						showNote={true}
						onClick={() => onActivityClick?.(item.data.id)}
						isSelected={selectedActivityId === item.data.id}
					/>
				{:else}
					<TrainingNoteListItemCompact
						note={item.data}
						onEdit={(content, date) => onNoteSave?.(item.data.id, content, date)}
						onDelete={() => onNoteDelete?.(item.data.id)}
					/>
				{/if}
			{/each}
		</div>
	{/if}

	{#if groupedItems.earlier.length > 0}
		<p class={containerClass}>Earlier</p>
		<div class="flex flex-col">
			{#each groupedItems.earlier as item}
				{#if item.type === 'activity'}
					<ActivitiesListItem
						activity={item.data}
						showNote={true}
						onClick={() => onActivityClick?.(item.data.id)}
						isSelected={selectedActivityId === item.data.id}
					/>
				{:else}
					<TrainingNoteListItemCompact
						note={item.data}
						onEdit={(content, date) => onNoteSave?.(item.data.id, content, date)}
						onDelete={() => onNoteDelete?.(item.data.id)}
					/>
				{/if}
			{/each}
		</div>
	{/if}

	<div class="flex items-center justify-between pt-2 text-lg font-semibold tracking-wide">
		<button class="btn btn-link btn-sm" onclick={moreCallback}> view all →</button>
	</div>

	{#if activityList.length === 0 && trainingNotes.length === 0}
		<div class="p-4 pb-2 text-center text-sm tracking-wide italic opacity-60">
			No activities or notes
		</div>
	{/if}
</div>
