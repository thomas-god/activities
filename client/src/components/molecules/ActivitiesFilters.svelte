<script lang="ts">
	import type { ActivityList } from '$lib/api';
	import { RPE_VALUES, getRpeColor } from '$lib/rpe';
	import { WORKOUT_TYPE_LABELS, getWorkoutTypeColor, type WorkoutType } from '$lib/workout-type';
	import {
		getSportCategory,
		sportCategoryDisplay,
		sportCategoryIcons,
		type SportCategory
	} from '$lib/sport';
	import { untrack } from 'svelte';

	let {
		activities,
		onFilterChange,
		initialRpe = [],
		initialWorkoutTypes = [],
		initialSportCategories = [],
		showNotes = $bindable(false),
		showNotesFilter = true,
		open = false,
		onFiltersStateChange
	}: {
		activities: ActivityList;
		onFilterChange: (filteredActivities: ActivityList) => void;
		initialRpe?: number[];
		initialWorkoutTypes?: WorkoutType[];
		initialSportCategories?: SportCategory[];
		showNotes?: boolean;
		onFiltersStateChange?: (filters: {
			rpe: number[];
			workoutTypes: WorkoutType[];
			sportCategories: SportCategory[];
			showNotes: boolean;
		}) => void;
		open?: boolean;
		showNotesFilter?: boolean;
	} = $props();

	// Filter state - initialize from props
	let selectedRpe = $state<number[]>(initialRpe);
	let selectedWorkoutTypes = $state<WorkoutType[]>(initialWorkoutTypes);
	let selectedSportCategories = $state<SportCategory[]>(initialSportCategories);

	// Update state when initial values change (e.g., when navigating back/forward)
	$effect(() => {
		selectedRpe = initialRpe;
		selectedWorkoutTypes = initialWorkoutTypes;
		selectedSportCategories = initialSportCategories;
	});

	// Get unique sport categories from activities
	let availableSportCategories = $derived.by(() => {
		const categories = new Set<SportCategory>();
		for (const activity of activities) {
			if (activity.sport_category) {
				categories.add(activity.sport_category);
			} else {
				const category = getSportCategory(activity.sport);
				if (category) categories.add(category);
			}
		}
		return Array.from(categories).sort();
	});

	// Filter activities
	let filteredActivities = $derived.by(() => {
		let filtered = activities;

		// Filter by RPE
		if (selectedRpe.length > 0) {
			filtered = filtered.filter((activity) => {
				return activity.rpe !== null && selectedRpe.includes(activity.rpe);
			});
		}

		// Filter by workout type
		if (selectedWorkoutTypes.length > 0) {
			filtered = filtered.filter((activity) => {
				return (
					activity.workout_type !== null && selectedWorkoutTypes.includes(activity.workout_type)
				);
			});
		}

		// Filter by sport category
		if (selectedSportCategories.length > 0) {
			filtered = filtered.filter((activity) => {
				const activityCategory = activity.sport_category || getSportCategory(activity.sport);
				return activityCategory !== null && selectedSportCategories.includes(activityCategory);
			});
		}

		return filtered;
	});

	// Notify parent when filters change
	$effect(() => {
		onFilterChange(filteredActivities);
	});

	// Helper function to notify URL change
	const notifyFiltersChange = () => {
		untrack(() => {
			if (onFiltersStateChange !== undefined) {
				onFiltersStateChange({
					rpe: selectedRpe,
					workoutTypes: selectedWorkoutTypes,
					sportCategories: selectedSportCategories,
					showNotes
				});
			}
		});
	};

	// Toggle functions
	const toggleRpe = (rpe: number) => {
		if (selectedRpe.includes(rpe)) {
			selectedRpe = selectedRpe.filter((r) => r !== rpe);
		} else {
			selectedRpe = [...selectedRpe, rpe];
		}
		notifyFiltersChange();
	};

	const toggleWorkoutType = (workoutType: WorkoutType) => {
		if (selectedWorkoutTypes.includes(workoutType)) {
			selectedWorkoutTypes = selectedWorkoutTypes.filter((wt) => wt !== workoutType);
		} else {
			selectedWorkoutTypes = [...selectedWorkoutTypes, workoutType];
		}
		notifyFiltersChange();
	};

	const toggleSportCategory = (category: SportCategory) => {
		if (selectedSportCategories.includes(category)) {
			selectedSportCategories = selectedSportCategories.filter((c) => c !== category);
		} else {
			selectedSportCategories = [...selectedSportCategories, category];
		}
		notifyFiltersChange();
	};

	const toggleShowNotes = () => {
		showNotes = !showNotes;
		notifyFiltersChange();
	};

	const clearFilters = () => {
		selectedRpe = [];
		selectedWorkoutTypes = [];
		selectedSportCategories = [];
		notifyFiltersChange();
	};

	let hasActiveFilters = $derived(
		selectedRpe.length > 0 || selectedWorkoutTypes.length > 0 || selectedSportCategories.length > 0
	);
</script>

<details class="collapse-arrow collapse" bind:open>
	<summary class="collapse-title flex items-center justify-between">
		<div class="flex items-center gap-2">
			<h2 class="text-lg font-semibold">Filters</h2>
			{#if hasActiveFilters}
				<span class="badge badge-sm badge-primary">
					{filteredActivities.length} / {activities.length}
				</span>
			{/if}
		</div>
	</summary>

	<div class=" collapse-content mx-4 pt-0">
		<div class="flex flex-col gap-4">
			<!-- Sport Category Filter -->
			{#if availableSportCategories.length > 0}
				<div>
					<div class="mb-2 text-sm font-medium">Sport</div>
					<div class="flex flex-wrap gap-2">
						{#each availableSportCategories as category}
							<button
								class={`btn btn-sm ${selectedSportCategories.includes(category) ? 'btn-primary' : 'btn-ghost'}`}
								onclick={() => toggleSportCategory(category)}
							>
								<span class="text-lg">{sportCategoryIcons[category]}</span>
								<span>{sportCategoryDisplay(category)}</span>
							</button>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Workout Type Filter -->
			<div>
				<div class="mb-2 text-sm font-medium">Workout Type</div>
				<div class="flex flex-wrap gap-2">
					{#each WORKOUT_TYPE_LABELS as { value, label }}
						<button
							class={`btn btn-sm ${selectedWorkoutTypes.includes(value) ? getWorkoutTypeColor(value) : 'btn-ghost'}`}
							onclick={() => toggleWorkoutType(value)}
						>
							{label}
						</button>
					{/each}
				</div>
			</div>

			<!-- RPE Filter -->
			<div>
				<div class="mb-2 text-sm font-medium">RPE (Rate of Perceived Exertion)</div>
				<div class="flex flex-wrap gap-2">
					{#each RPE_VALUES as rpe}
						<button
							class={`btn btn-sm ${selectedRpe.includes(rpe) ? getRpeColor(rpe) : 'btn-ghost'}`}
							onclick={() => toggleRpe(rpe)}
						>
							{rpe}
						</button>
					{/each}
				</div>
			</div>

			<!-- Show Notes Toggle -->
			{#if showNotesFilter}
				<div>
					<div class="mb-2 text-sm font-medium">Display</div>
					<button
						class={`btn btn-sm ${showNotes ? 'btn-primary' : 'btn-ghost'}`}
						onclick={toggleShowNotes}
					>
						📝 {showNotes ? 'Hide' : 'Show'} Notes
					</button>
				</div>
			{/if}

			{#if hasActiveFilters}
				<div>
					<button
						class="btn btn-sm"
						onclick={(e) => {
							e.preventDefault();
							clearFilters();
						}}
					>
						Clear all filters
					</button>
				</div>
			{/if}
		</div>
	</div>
</details>

<style>
	.rpe-easy {
		background-color: var(--color-rpe-easy);
		color: var(--color-rpe-easy-text);
	}

	.rpe-moderate {
		background-color: var(--color-rpe-moderate);
		color: var(--color-rpe-moderate-text);
	}

	.rpe-hard {
		background-color: var(--color-rpe-hard);
		color: var(--color-rpe-hard-text);
	}

	.rpe-very-hard {
		background-color: var(--color-rpe-very-hard);
		color: var(--color-rpe-very-hard-text);
	}

	.rpe-max {
		background-color: var(--color-rpe-max);
		color: var(--color-rpe-max-text);
	}

	.workout-easy {
		background-color: var(--color-workout-easy);
		color: var(--color-workout-easy-text);
	}

	.workout-tempo {
		background-color: var(--color-workout-tempo);
		color: var(--color-workout-tempo-text);
	}

	.workout-intervals {
		background-color: var(--color-workout-intervals);
		color: var(--color-workout-intervals-text);
	}

	.workout-long-run {
		background-color: var(--color-workout-long-run);
		color: var(--color-workout-long-run-text);
	}

	.workout-race {
		background-color: var(--color-workout-race);
		color: var(--color-workout-race-text);
	}

	.workout-cross-training {
		background-color: var(--color-workout-cross-training);
		color: var(--color-workout-cross-training-text);
	}
</style>
