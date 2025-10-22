<script lang="ts">
	import type { ActivityList } from '$lib/api';
	import { RPE_VALUES, getRpeColor } from '$lib/rpe';
	import { WORKOUT_TYPE_LABELS, getWorkoutTypeColor, type WorkoutType } from '$lib/workout-type';
	import { getSportCategory, sportCategoryIcons, type SportCategory } from '$lib/sport';

	let {
		activities,
		onFilterChange
	}: {
		activities: ActivityList;
		onFilterChange: (filteredActivities: ActivityList) => void;
	} = $props();

	// Filter state
	let selectedRpe = $state<number[]>([]);
	let selectedWorkoutTypes = $state<WorkoutType[]>([]);
	let selectedSportCategories = $state<SportCategory[]>([]);

	// Show/hide filters
	let showFilters = $state(false);

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

	// Toggle functions
	const toggleRpe = (rpe: number) => {
		if (selectedRpe.includes(rpe)) {
			selectedRpe = selectedRpe.filter((r) => r !== rpe);
		} else {
			selectedRpe = [...selectedRpe, rpe];
		}
	};

	const toggleWorkoutType = (workoutType: WorkoutType) => {
		if (selectedWorkoutTypes.includes(workoutType)) {
			selectedWorkoutTypes = selectedWorkoutTypes.filter((wt) => wt !== workoutType);
		} else {
			selectedWorkoutTypes = [...selectedWorkoutTypes, workoutType];
		}
	};

	const toggleSportCategory = (category: SportCategory) => {
		if (selectedSportCategories.includes(category)) {
			selectedSportCategories = selectedSportCategories.filter((c) => c !== category);
		} else {
			selectedSportCategories = [...selectedSportCategories, category];
		}
	};

	const clearFilters = () => {
		selectedRpe = [];
		selectedWorkoutTypes = [];
		selectedSportCategories = [];
	};

	let hasActiveFilters = $derived(
		selectedRpe.length > 0 || selectedWorkoutTypes.length > 0 || selectedSportCategories.length > 0
	);
</script>

<div class="flex items-center justify-between">
	<div class="flex items-center gap-2">
		<h2 class="text-lg font-semibold">Filters</h2>
		{#if hasActiveFilters}
			<span class="badge badge-sm badge-primary">
				{filteredActivities.length} / {activities.length}
			</span>
		{/if}
	</div>
	<div class="flex items-center gap-2">
		{#if hasActiveFilters}
			<button class="btn btn-ghost btn-sm" onclick={clearFilters}> Clear all </button>
		{/if}
		<button
			class="btn btn-ghost btn-sm"
			onclick={() => (showFilters = !showFilters)}
			aria-expanded={showFilters}
		>
			{showFilters ? '▲' : '▼'}
		</button>
	</div>
</div>

{#if showFilters}
	<div class="mt-4 flex flex-col gap-4">
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
							<span>{category}</span>
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
	</div>
{/if}

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
</style>
