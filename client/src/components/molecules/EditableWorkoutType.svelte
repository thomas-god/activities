<script lang="ts">
	import {
		WORKOUT_TYPE_LABELS,
		getWorkoutTypeLabel,
		getWorkoutTypeColor,
		type WorkoutType
	} from '$lib/workout-type';

	let {
		workoutType: initialWorkoutType,
		editCallback
	}: {
		workoutType: WorkoutType | null;
		editCallback: (newWorkoutType: WorkoutType | null) => Promise<void>;
	} = $props();

	let workoutType = $state(initialWorkoutType);
	let editMode = $state(false);

	const handleSave = () => {
		editMode = false;
		editCallback(workoutType);
	};

	const handleCancel = () => {
		editMode = false;
		workoutType = initialWorkoutType;
	};
</script>

{#if editMode}
	<div class="flex flex-col gap-2">
		<div class="text-sm font-medium">Workout Type</div>
		<div class="flex flex-wrap gap-2">
			<button
				class={`btn btn-sm ${workoutType === null ? 'btn-active' : 'btn-ghost'}`}
				onclick={() => (workoutType = null)}
			>
				Clear
			</button>
			{#each WORKOUT_TYPE_LABELS as type}
				<button
					class={`btn btn-sm ${workoutType === type.value ? `btn-active ${getWorkoutTypeColor(type.value)}` : 'btn-ghost'}`}
					onclick={() => (workoutType = type.value)}
				>
					{type.label}
				</button>
			{/each}
		</div>
		<div class="flex gap-2">
			<button class="btn btn-sm btn-primary" onclick={handleSave}>💾 Save</button>
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-2">
		<div class="flex flex-row text-sm font-medium">
			<span> Workout Type </span>
			{#if workoutType !== null}
				<button class="btn ml-auto btn-ghost btn-xs" onclick={() => (editMode = true)}>
					✏️ Edit
				</button>
			{/if}
		</div>
		{#if workoutType === null}
			<button class="mr-auto link text-sm link-hover opacity-70" onclick={() => (editMode = true)}>
				Add workout type
			</button>
		{:else}
			<div class="flex items-center gap-2">
				<span class={`badge workout-${workoutType}`}>
					{getWorkoutTypeLabel(workoutType)}
				</span>
			</div>
		{/if}
	</div>
{/if}

<style>
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

	.workout-long_run {
		background-color: var(--color-workout-long-run);
		color: var(--color-workout-long-run-text);
	}

	.workout-race {
		background-color: var(--color-workout-race);
		color: var(--color-workout-race-text);
	}
</style>
