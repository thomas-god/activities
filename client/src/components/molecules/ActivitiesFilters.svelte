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
		filters = $bindable(),
		open = false
	}: {
		activities: ActivityList;
		filters: {
			rpe: number[];
			workoutTypes: WorkoutType[];
			sportCategories: SportCategory[];
		};
		open?: boolean;
	} = $props();

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

	// // Notify parent when filters change
	// $effect(() => {
	// 	onFilterChange(filteredActivities);
	// });

	// // Helper function to notify URL change
	// const notifyFiltersChange = () => {
	// 	untrack(() => {
	// 		if (onFiltersStateChange !== undefined) {
	// 			onFiltersStateChange({
	// 				rpe: selectedRpe,
	// 				workoutTypes: selectedWorkoutTypes,
	// 				sportCategories: selectedSportCategories
	// 			});
	// 		}
	// 	});
	// };

	// Toggle functions
	const toggleRpe = (rpe: number) => {
		if (filters.rpe.includes(rpe)) {
			filters = { ...filters, rpe: filters.rpe.filter((r) => r !== rpe) };
		} else {
			filters = { ...filters, rpe: [...filters.rpe, rpe] };
		}
	};

	const toggleWorkoutType = (workoutType: WorkoutType) => {
		if (filters.workoutTypes.includes(workoutType)) {
			filters = { ...filters, workoutTypes: filters.workoutTypes.filter((r) => r !== workoutType) };
		} else {
			filters = { ...filters, workoutTypes: [...filters.workoutTypes, workoutType] };
		}
	};

	const toggleSportCategory = (category: SportCategory) => {
		if (filters.sportCategories.includes(category)) {
			filters = {
				...filters,
				sportCategories: filters.sportCategories.filter((r) => r !== category)
			};
		} else {
			filters = { ...filters, sportCategories: [...filters.sportCategories, category] };
		}
	};

	const clearFilters = () => {
		filters = {
			rpe: [],
			sportCategories: [],
			workoutTypes: []
		};
	};

	let hasActiveFilters = $derived(
		filters.rpe.length > 0 || filters.workoutTypes.length > 0 || filters.sportCategories.length > 0
	);
</script>

<details class="collapse-arrow collapse" bind:open>
	<summary class="collapse-title flex items-center justify-between">
		<div class="flex items-center gap-2">
			<h2 class="text-lg font-semibold">Filters</h2>
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
								class={`btn btn-sm ${filters.sportCategories.includes(category) ? 'btn-primary' : 'btn-ghost'}`}
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
							class={`btn btn-sm ${filters.workoutTypes.includes(value) ? getWorkoutTypeColor(value) : 'btn-ghost'}`}
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
							class={`btn btn-sm ${filters.rpe.includes(rpe) ? getRpeColor(rpe) : 'btn-ghost'}`}
							onclick={() => toggleRpe(rpe)}
						>
							{rpe}
						</button>
					{/each}
				</div>
			</div>

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
