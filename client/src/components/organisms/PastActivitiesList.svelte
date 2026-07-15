<script lang="ts">
	import { dayjs } from '$lib/duration';
	import type { ActivityList, ActivityListSummaryItems } from '$lib/api';
	import type { TrainingNotesList } from '$lib/api/training';
	import ActivityListComponent, { type TimelineItem } from './ActivityList.svelte';

	let {
		activityList,
		trainingNotes = [],
		moreCallback,
		onActivityClick,
		activityListFormat,
		selectedActivityId = null
	}: {
		activityList: ActivityList;
		trainingNotes?: TrainingNotesList;
		moreCallback: () => void;
		onActivityClick: (activityId: string) => void;
		selectedActivityId?: string | null;
		activityListFormat: ActivityListSummaryItems;
	} = $props();

	let sorted_activities = $derived(
		activityList.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	let timeline = $derived.by(() => {
		const now = dayjs();
		const thisWeek = [] as TimelineItem[];
		const thisMonth = [] as TimelineItem[];
		const earlier = [] as TimelineItem[];

		// Add activities to timeline
		for (const activity of sorted_activities) {
			const date = dayjs(activity.start_time);
			const item: TimelineItem = {
				type: 'activity',
				data: activity,
				date: activity.start_time
			};

			if (date > now.startOf('isoWeek')) {
				thisWeek.push(item);
			} else if (date > now.startOf('month')) {
				thisMonth.push(item);
			} else {
				earlier.push(item);
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
				thisWeek.push(item);
			} else if (date > now.startOf('month')) {
				thisMonth.push(item);
			} else {
				earlier.push(item);
			}
		}

		// Sort each group by date (most recent first)
		thisWeek.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));
		thisMonth.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));
		earlier.sort((a, b) => dayjs(b.date).diff(dayjs(a.date)));

		const timeline = new Map();
		timeline.set('This week', thisWeek);
		timeline.set('This month', thisMonth);
		timeline.set('Earlier', earlier);
		return timeline;
	});
</script>

<div class="rounded-box bg-base-100 p-4 shadow-md">
	<div class="flex items-center justify-between pb-2 text-lg font-semibold tracking-wide">
		<span> Recent activities and notes</span>
		<button class="btn btn-link btn-sm" onclick={moreCallback}> view all →</button>
	</div>

	<ActivityListComponent
		{selectedActivityId}
		selectActivityCallback={onActivityClick}
		{activityListFormat}
		{timeline}
		showGroupNumberOfActivities={false}
	/>

	{#if activityList.length !== 0 || trainingNotes.length !== 0}
		<div class="flex items-center justify-between pt-2 text-lg font-semibold tracking-wide">
			<button class="btn btn-link btn-sm" onclick={moreCallback}> view all →</button>
		</div>
	{/if}

	{#if activityList.length === 0 && trainingNotes.length === 0}
		<div class="p-4 pb-2 text-center text-sm tracking-wide italic opacity-60">
			No activities or notes
		</div>
	{/if}
</div>
