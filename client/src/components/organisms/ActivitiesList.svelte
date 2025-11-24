<script lang="ts">
	import type { ActivityList, ActivityListItem } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import ActivitiesListItem from './ActivitiesListItem.svelte';
	import ActivitiesFilters from '../molecules/ActivitiesFilters.svelte';
	import type { WorkoutType } from '$lib/workout-type';
	import type { SportCategory } from '$lib/sport';

	let {
		activityList,
		initialRpe = [],
		initialWorkoutTypes = [],
		initialSportCategories = [],
		initialShowNotes = false,
		onFiltersChange,
		selectedActivity = null,
		onActivitySelected
	}: {
		activityList: ActivityList;
		initialRpe?: number[];
		initialWorkoutTypes?: WorkoutType[];
		initialSportCategories?: SportCategory[];
		initialShowNotes?: boolean;
		onFiltersChange?: (filters: {
			rpe: number[];
			workoutTypes: WorkoutType[];
			sportCategories: SportCategory[];
			showNotes: boolean;
		}) => void;
		selectedActivity: string | null;
		onActivitySelected?: (activityId: string) => void;
	} = $props();

	let filteredActivityList = $state<ActivityList>(activityList);
	let showNotes = $state<boolean>(initialShowNotes);

	let historyStartMonth = $derived(dayjs(filteredActivityList.at(-1)?.start_time).startOf('month'));
	let historyEndMonth = dayjs().startOf('month');

	let activitiesByMonth = $derived.by(() => {
		const activities: Map<string, ActivityListItem[]> = new Map();

		let date = historyEndMonth;
		while (date >= historyStartMonth) {
			activities.set(date.format('MMMM YYYY'), []);
			date = date.subtract(1, 'month');
		}

		for (const activity of filteredActivityList) {
			let activityStart = dayjs(activity.start_time).format('MMMM YYYY');
			activities.get(activityStart)?.push(activity);
		}

		return activities;
	});

	const handleFilterChange = (filtered: ActivityList) => {
		filteredActivityList = filtered;
	};

	const handleFiltersStateChange = (filters: {
		rpe: number[];
		workoutTypes: WorkoutType[];
		sportCategories: SportCategory[];
		showNotes: boolean;
	}) => {
		showNotes = filters.showNotes;
		onFiltersChange?.(filters);
	};

	const activityClickedCallback = (activityId: string) => {
		if (onActivitySelected !== undefined) {
			onActivitySelected(activityId);
		}
	};
</script>

<div class=" rounded-box bg-base-100 shadow-md">
	<ActivitiesFilters
		activities={activityList}
		onFilterChange={handleFilterChange}
		{initialRpe}
		{initialWorkoutTypes}
		{initialSportCategories}
		bind:showNotes
		onFiltersStateChange={handleFiltersStateChange}
	/>
	<div class="flex flex-col p-4 pt-0">
		{#if filteredActivityList.length === 0}
			<div class="py-8 text-center text-base-content/60">
				No activities match the selected filters
			</div>
		{:else}
			{#each activitiesByMonth as [month, activities]}
				{#if activities.length > 0}
					<div class="flex flex-col">
						<div class="my-3 text-xs font-semibold tracking-wide text-base-content/60 uppercase">
							{month} - {activities.length} activities
						</div>
						{#each activities as activity}
							<ActivitiesListItem
								{activity}
								showNote={showNotes}
								isSelected={activity.id === selectedActivity}
								onClick={() => activityClickedCallback(activity.id)}
							/>
						{/each}
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>
