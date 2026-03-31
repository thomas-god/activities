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
	import { emptyFilters, filterActivities, type ActivitiesFilters } from '$lib/filters';
	import type { RangeFilter } from '$lib/filters';

	let {
		activities,
		filteredActivities = $bindable(),
		filters = $bindable(),
		showLabel = true
	}: {
		activities: ActivityList;
		filteredActivities: ActivityList;
		filters: ActivitiesFilters;
		showLabel?: boolean;
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
		filters = emptyFilters();
	};

	// Compute per-field min/max from the activity list for placeholder hints
	let activityDurationRange = $derived.by(() => {
		const values = activities
			.map((a) => a.statistics['Duration'])
			.filter((v): v is number => v !== undefined);
		if (values.length === 0) return null;
		return { min: Math.min(...values) / 60, max: Math.max(...values) / 60 };
	});

	let activityDistanceRange = $derived.by(() => {
		const values = activities
			.map((a) => a.statistics['Distance'])
			.filter((v): v is number => v !== undefined);
		if (values.length === 0) return null;
		return { min: Math.min(...values) / 1000, max: Math.max(...values) / 1000 };
	});

	let activityElevationRange = $derived.by(() => {
		const values = activities
			.map((a) => a.statistics['Elevation'])
			.filter((v): v is number => v !== undefined);
		if (values.length === 0) return null;
		return { min: Math.min(...values), max: Math.max(...values) };
	});

	const setDurationRange = (range: RangeFilter) => {
		filters = { ...filters, durationRange: range };
	};
	const setDistanceRange = (range: RangeFilter) => {
		filters = { ...filters, distanceRange: range };
	};
	const setElevationRange = (range: RangeFilter) => {
		filters = { ...filters, elevationRange: range };
	};

	let hasActiveFilters = $derived(
		filters.rpe.length > 0 ||
			filters.workoutTypes.length > 0 ||
			filters.sportCategories.length > 0 ||
			filters.durationRange.min !== null ||
			filters.durationRange.max !== null ||
			filters.distanceRange.min !== null ||
			filters.distanceRange.max !== null ||
			filters.elevationRange.min !== null ||
			filters.elevationRange.max !== null
	);
</script>

<button
	onclick={() => dialogElement.showModal()}
	class={`btn ${hasActiveFilters ? 'btn-warning' : 'btn-ghost'} btn-sm`}
	><img src="/icons/filter.svg" alt="Filter icon" class="h-5 w-5" />
	{#if showLabel}
		<span class="ml-1 hidden sm:inline">Filters</span>
	{/if}
</button>

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

		<div class="mx-2 pt-0">
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

				<!-- Duration Range Filter -->
				<div>
					<div class="mb-2 text-sm font-medium">Duration (minutes)</div>
					<div class="flex items-center gap-2">
						<input
							type="number"
							min="0"
							placeholder={activityDurationRange !== null
								? Math.floor(activityDurationRange.min).toString()
								: 'Min'}
							value={filters.durationRange.min !== null ? filters.durationRange.min / 60 : ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setDurationRange({
									...filters.durationRange,
									min: isNaN(v) ? null : v * 60
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
						<span class="text-sm text-base-content/60">—</span>
						<input
							type="number"
							min="0"
							placeholder={activityDurationRange !== null
								? Math.ceil(activityDurationRange.max).toString()
								: 'Max'}
							value={filters.durationRange.max !== null ? filters.durationRange.max / 60 : ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setDurationRange({
									...filters.durationRange,
									max: isNaN(v) ? null : v * 60
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
					</div>
				</div>

				<!-- Distance Range Filter -->
				<div>
					<div class="mb-2 text-sm font-medium">Distance (km)</div>
					<div class="flex items-center gap-2">
						<input
							type="number"
							min="0"
							step="1"
							placeholder={activityDistanceRange !== null
								? activityDistanceRange.min.toFixed(0)
								: 'Min'}
							value={filters.distanceRange.min !== null
								? (filters.distanceRange.min / 1000).toFixed(0)
								: ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setDistanceRange({
									...filters.distanceRange,
									min: isNaN(v) ? null : v * 1000
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
						<span class="text-sm text-base-content/60">—</span>
						<input
							type="number"
							min="0"
							step="1"
							placeholder={activityDistanceRange !== null
								? activityDistanceRange.max.toFixed(0)
								: 'Max'}
							value={filters.distanceRange.max !== null
								? (filters.distanceRange.max / 1000).toFixed(0)
								: ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setDistanceRange({
									...filters.distanceRange,
									max: isNaN(v) ? null : v * 1000
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
					</div>
				</div>

				<!-- Elevation Range Filter -->
				<div>
					<div class="mb-2 text-sm font-medium">Elevation gained (m)</div>
					<div class="flex items-center gap-2">
						<input
							type="number"
							min="0"
							placeholder={activityElevationRange !== null
								? Math.floor(activityElevationRange.min).toString()
								: 'Min'}
							value={filters.elevationRange.min !== null ? filters.elevationRange.min : ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setElevationRange({
									...filters.elevationRange,
									min: isNaN(v) ? null : v
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
						<span class="text-sm text-base-content/60">—</span>
						<input
							type="number"
							min="0"
							placeholder={activityElevationRange !== null
								? Math.ceil(activityElevationRange.max).toString()
								: 'Max'}
							value={filters.elevationRange.max !== null ? filters.elevationRange.max : ''}
							oninput={(e) => {
								const v = e.currentTarget.valueAsNumber;
								setElevationRange({
									...filters.elevationRange,
									max: isNaN(v) ? null : v
								});
							}}
							class="input-bordered input input-sm w-28"
						/>
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
