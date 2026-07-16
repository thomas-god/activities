<script lang="ts">
	import type { ActivityList, ActivityListSummaryItems, TrainingNotesList } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import ActivityListComponent, {
		type TimelineItem
	} from '$components/organisms/activityList/ActivityList.svelte';

	let {
		activities,
		notes,
		selectedActivityId,
		selectActivityCallback,
		noteChangedCallback,
		activityListFormat,
		endDate = null
	}: {
		activities: ActivityList;
		notes: TrainingNotesList;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		noteChangedCallback: () => void;
		endDate?: string | null;
		activityListFormat: ActivityListSummaryItems;
	} = $props();

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

	const timelineByMonth: Map<string, TimelineItem[]> = $derived.by(() => {
		let timelineStartMonth = dayjs(timeline.at(-1)?.date).startOf('month');
		let timelineEndMonth =
			endDate === null ? dayjs().startOf('month') : dayjs(endDate).startOf('month');

		const timelineByMonth = new Map();
		let date = timelineEndMonth;
		while (date >= timelineStartMonth) {
			timelineByMonth.set(date.format('MMMM YYYY'), []);
			date = date.subtract(1, 'month');
		}

		for (const item of timeline) {
			let start = dayjs(item.date).format('MMMM YYYY');
			timelineByMonth.get(start)?.push(item);
		}

		return timelineByMonth;
	});
</script>

<ActivityListComponent
	{selectedActivityId}
	{selectActivityCallback}
	{activityListFormat}
	timeline={timelineByMonth}
	{noteChangedCallback}
/>
