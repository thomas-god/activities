<script lang="ts">
	import type { ActivityList, ActivityListSummaryItems, TrainingNotesList } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { isNone, type Option } from '$lib/Options';
	import ActivityListComponent, {
		type TimelineItem
	} from '$ui/activity/internal/ActivityList.svelte';
	import type { SearchResult } from '$ui/shared/SearchField.svelte';
	import { SvelteMap } from 'svelte/reactivity';

	let {
		activities,
		notes,
		searchResults,
		selectedActivityId,
		selectActivityCallback,
		noteChangedCallback,
		activityListFormat,
		renderByChunk,
		endDate = null
	}: {
		activities: ActivityList;
		notes: TrainingNotesList;
		searchResults: Option<SearchResult[]>;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		noteChangedCallback: () => void;
		endDate?: string | null;
		renderByChunk: boolean;
		activityListFormat: ActivityListSummaryItems;
	} = $props();

	const filterItem = (kind: SearchResult['kind'], item: { id: string }) => {
		if (isNone(searchResults)) {
			return true;
		}
		return searchResults.value.some((res) => res.kind === kind && res.id === item.id);
	};

	const timeline = $derived.by((): TimelineItem[] => {
		const items = [
			...activities.map((activity) => ({
				type: 'activity' as const,
				data: activity,
				date: activity.start_time,
				found: filterItem('activity', activity)
			})),
			...notes.map((note) => ({
				type: 'note' as const,
				data: note,
				date: note.date,
				found: filterItem('training_note', note)
			}))
		];

		return items.sort((a, b) => (a.date > b.date ? -1 : 1)).filter((item) => item.found);
	});

	const timelineByMonth: SvelteMap<string, TimelineItem[]> = $derived.by(() => {
		let timelineStartMonth = dayjs(timeline.at(-1)?.date).startOf('month');
		let timelineEndMonth =
			endDate === null ? dayjs().startOf('month') : dayjs(endDate).startOf('month');

		const timelineByMonth: SvelteMap<string, TimelineItem[]> = new SvelteMap();
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
	{renderByChunk}
/>
