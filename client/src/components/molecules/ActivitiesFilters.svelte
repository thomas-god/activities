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
	import { filterActivities, type ActivitiesFilters } from '$lib/filters';

	let {
		activities,
		filteredActivities = $bindable(),
		filters = $bindable()
	}: {
		activities: ActivityList;
		filteredActivities: ActivityList;
		filters: ActivitiesFilters;
	} = $props();

	let dialogElement: HTMLDialogElement;

	$effect(() => {
		filteredActivities = filterActivities(activities, filters);
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

<button
	onclick={() => dialogElement.showModal()}
	class={`btn ${hasActiveFilters ? 'btn-warning' : 'btn-ghost'} btn-sm`}
	><img src="/icons/filter.svg" alt="Filter icon" class="h-5 w-5" />
	<span class="ml-1 hidden sm:inline">Filters</span></button
>

<dialog class="modal" bind:this={dialogElement}>
	<div class="modal-box">
		<summary class="collapse-title flex items-center justify-between">
			<div class="flex items-center gap-2">
				<h2 class="text-lg font-semibold">
					Filters
					{#if hasActiveFilters}
						<span class="text-sm font-light">
							({filteredActivities.length}/{activities.length} activities)
						</span>
					{/if}
				</h2>
			</div>
		</summary>

		<div class="mx-4 pt-0">
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
									<img
										src={`/icons/${sportCategoryIcons[category]}`}
										class="h-6 w-6"
										alt="Menu icon"
									/>
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

		<div class="modal-action">
			<form method="dialog">
				<button class="btn" onclick={() => dialogElement.close()}>Close</button>
			</form>
		</div>
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

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
