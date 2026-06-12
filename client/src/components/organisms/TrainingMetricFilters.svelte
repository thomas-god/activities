<script lang="ts">
	import type { Sport, SportCategory } from '$lib/sport';
	import { isNone, isSome, type Option, some } from '$lib/Options';
	import type { RPEValue } from '$lib/rpe';
	import type { BonkStatus } from '$lib/nutrition';
	import RpeFilterV2 from '$components/molecules/RpeFilterV2.svelte';
	import BonkStatusFilterV2 from '$components/molecules/BonkStatusFilterV2.svelte';
	import type { WorkoutType } from '$lib/workout-type';
	import WorkoutTypeFilterV2 from '$components/molecules/WorkoutTypeFilterV2.svelte';
	import SportFilterV2 from '$components/molecules/SportFilterV2.svelte';

	export interface TrainingMetricFiltersType {
		sports: Option<Sport[]>;
		sportCategories: Option<SportCategory[]>;
		rpes: Option<RPEValue[]>;
		workoutTypes: Option<WorkoutType[]>;
		bonked: Option<BonkStatus>;
	}

	let { filters = $bindable() }: { filters: TrainingMetricFiltersType } = $props();

	let allFiltersSet = $derived(
		isSome(filters.sports) &&
			isSome(filters.sportCategories) &&
			isSome(filters.rpes) &&
			isSome(filters.workoutTypes) &&
			isSome(filters.bonked)
	);
</script>

<label class="label" for="metric-name"
	>Add filters

	{#if !allFiltersSet}
		<button class="btn btn-circle btn-xs" popovertarget="popover-1" style="anchor-name:--anchor-1">
			<img src="/icons/plus.svg" alt="Plus sign" class="inline h-5 w-5" />
		</button>
		<ul
			class="menu dropdown z-1 w-52 rounded-box bg-base-100 p-2 shadow-sm"
			popover
			id="popover-1"
			style="position-anchor:--anchor-1"
		>
			{#if isNone(filters.sports) && isNone(filters.sportCategories)}
				<li>
					<button
						onclick={() => {
							filters.sports = some([]);
							filters.sportCategories = some([]);
						}}>Sports</button
					>
				</li>
			{/if}

			{#if isNone(filters.rpes)}
				<li>
					<button onclick={() => (filters.rpes = some([]))}>RPE values</button>
				</li>
			{/if}

			{#if isNone(filters.workoutTypes)}
				<li>
					<button onclick={() => (filters.workoutTypes = some([]))}>Workout types</button>
				</li>
			{/if}

			{#if isNone(filters.bonked)}
				<li>
					<button onclick={() => (filters.bonked = some('none'))}>Bonk status</button>
				</li>
			{/if}
		</ul>
	{/if}
</label>
<div class="flex flex-col gap-2 pl-2">
	<SportFilterV2
		bind:sports={() => filters.sports, (v) => (filters.sports = v)}
		bind:categories={() => filters.sportCategories, (v) => (filters.sportCategories = v)}
	/>

	<RpeFilterV2 bind:rpes={() => filters.rpes, (v) => (filters.rpes = v)} />

	<WorkoutTypeFilterV2
		bind:workoutTypes={() => filters.workoutTypes, (v) => (filters.workoutTypes = v)}
	/>

	<BonkStatusFilterV2 bind:bonkStatus={() => filters.bonked, (v) => (filters.bonked = v)} />
</div>
