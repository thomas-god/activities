<script lang="ts">
	import type { Activity, ActivityList, TrainingNote, TrainingNotesList } from '$lib/api';
	import ActivitiesListItem from '$components/organisms/ActivitiesListItem.svelte';
	import TrainingNoteListItemCompact from '$components/organisms/TrainingNoteListItemCompact.svelte';
	import { dayjs } from '$lib/duration';
	import ActivitiesFilters from '$components/molecules/ActivitiesFilters.svelte';
	import type { WorkoutType } from '$lib/workout-type';
	import { getSportCategory, type SportCategory } from '$lib/sport';

	let {
		activities,
		notes,
		selectedActivityId,
		selectActivityCallback,
		filters = $bindable(),
		endDate = null
	}: {
		activities: ActivityList;
		notes: TrainingNotesList;
		filters: {
			rpe: number[];
			workoutTypes: WorkoutType[];
			sportCategories: SportCategory[];
		};
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		endDate?: string | null;
	} = $props();

	let filteredActivities = $derived.by(() => {
		let filtered = activities;

		// Filter by RPE
		if (filters.rpe.length > 0) {
			filtered = filtered.filter((activity) => {
				return activity.rpe !== null && filters.rpe.includes(activity.rpe);
			});
		}

		// Filter by workout type
		if (filters.workoutTypes.length > 0) {
			filtered = filtered.filter((activity) => {
				return (
					activity.workout_type !== null && filters.workoutTypes.includes(activity.workout_type)
				);
			});
		}

		// Filter by sport category
		if (filters.sportCategories.length > 0) {
			filtered = filtered.filter((activity) => {
				const activityCategory = activity.sport_category || getSportCategory(activity.sport);
				return activityCategory !== null && filters.sportCategories.includes(activityCategory);
			});
		}

		return filtered;
	});

	// Merge activities and notes, sorted by date (most recent first)
	type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	const timeline = $derived.by((): TimelineItem[] => {
		const items: TimelineItem[] = [
			...filteredActivities.map((activity) => ({
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

<ActivitiesFilters {activities} bind:filters />
{#if timeline.length > 0}
	<div class="flex flex-col gap-0">
		{#each timelineByMonth as [month, items]}
			<div class="flex flex-col">
				<div class="my-3 text-xs font-semibold tracking-wide text-base-content/60 uppercase">
					{month} - {items.filter((item) => item.type === 'activity').length} activities
				</div>
				{#each items as item}
					{#if item.type === 'activity'}
						<ActivitiesListItem
							activity={item.data}
							onClick={() => selectActivityCallback(item.data.id)}
							isSelected={selectedActivityId === item.data.id}
						/>
					{:else}
						<TrainingNoteListItemCompact note={item.data} />
					{/if}
				{/each}
			</div>
		{/each}
	</div>
{:else}
	<div class="py-8 text-center text-sm italic opacity-70">
		No activities in this training period yet
	</div>
{/if}
