<script lang="ts">
	import type { Sport, SportCategory } from '#lib/sport.js';
	import { isNone, isSome, none, type Option, some } from '#lib/Options.js';
	import type { RPEValue } from '#lib/rpe.js';
	import type { BonkStatus } from '#lib/nutrition.js';
	import RpeFilter from './RpeFilter.svelte';
	import BonkStatusFilter from './BonkStatusFilter.svelte';
	import type { WorkoutType } from '#lib/workout-type.js';
	import WorkoutTypeFilter from './WorkoutTypeFilter.svelte';
	import SportFilter from '#ui/shared/SportFilter.svelte';
	import { Plus } from '@lucide/svelte';

	export interface TrainingMetricFiltersType {
		sports: Option<Sport[]>;
		sportCategories: Option<SportCategory[]>;
		rpes: Option<RPEValue[]>;
		workoutTypes: Option<WorkoutType[]>;
		bonked: Option<BonkStatus>;
	}

	let {
		filters = $bindable(),
		existingSportsConstraints = none()
	}: {
		filters: TrainingMetricFiltersType;
		existingSportsConstraints?: Option<{ sports: Sport[]; categories: SportCategory[] }>;
	} = $props();

	let allFiltersSet = $derived(
		isSome(filters.sports) &&
			isSome(filters.sportCategories) &&
			isSome(filters.rpes) &&
			isSome(filters.workoutTypes) &&
			isSome(filters.bonked)
	);

	let menuOpen = $state(false);
	let menuContainer: HTMLDivElement | undefined = $state();

	const handleWindowClick = (event: MouseEvent) => {
		if (menuOpen && menuContainer && !menuContainer.contains(event.target as Node)) {
			menuOpen = false;
		}
	};

	const handleWindowKeydown = (event: KeyboardEvent) => {
		if (event.key === 'Escape') {
			menuOpen = false;
		}
	};
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="label">
	Filter activities by

	{#if !allFiltersSet}
		<div class="relative inline-flex" bind:this={menuContainer}>
			<button
				type="button"
				class="btn btn-circle btn-xs"
				aria-haspopup="true"
				aria-expanded={menuOpen}
				onclick={() => (menuOpen = !menuOpen)}
			>
				<Plus class="size-5" />
			</button>
			{#if menuOpen}
				<ul
					class="menu absolute top-full left-0 z-50 mt-1 w-52 rounded-box bg-base-100 p-2 shadow-md"
				>
					{#if isNone(filters.sports) && isNone(filters.sportCategories)}
						<li>
							<button
								type="button"
								onclick={() => {
									filters.sports = some([]);
									filters.sportCategories = some([]);
									menuOpen = false;
								}}>Sports</button
							>
						</li>
					{/if}

					{#if isNone(filters.rpes)}
						<li>
							<button
								type="button"
								onclick={() => {
									filters.rpes = some([]);
									menuOpen = false;
								}}>RPE values</button
							>
						</li>
					{/if}

					{#if isNone(filters.workoutTypes)}
						<li>
							<button
								type="button"
								onclick={() => {
									filters.workoutTypes = some([]);
									menuOpen = false;
								}}>Workout types</button
							>
						</li>
					{/if}

					{#if isNone(filters.bonked)}
						<li>
							<button
								type="button"
								onclick={() => {
									filters.bonked = some('none');
									menuOpen = false;
								}}>Bonk status</button
							>
						</li>
					{/if}
				</ul>
			{/if}
		</div>
	{/if}
</div>
<div class="flex flex-col gap-2 pl-2">
	<SportFilter
		bind:sports={() => filters.sports, (v) => (filters.sports = v)}
		bind:categories={() => filters.sportCategories, (v) => (filters.sportCategories = v)}
		{existingSportsConstraints}
	/>

	<RpeFilter bind:rpes={() => filters.rpes, (v) => (filters.rpes = v)} />

	<WorkoutTypeFilter
		bind:workoutTypes={() => filters.workoutTypes, (v) => (filters.workoutTypes = v)}
	/>

	<BonkStatusFilter bind:bonkStatus={() => filters.bonked, (v) => (filters.bonked = v)} />
</div>
