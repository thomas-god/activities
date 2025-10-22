<script lang="ts">
	import type { WorkoutType } from '$lib/api';

	let {
		workoutType: initialWorkoutType,
		editCallback
	}: {
		workoutType: WorkoutType | null;
		editCallback: (newWorkoutType: WorkoutType | null) => Promise<void>;
	} = $props();

	let workoutType = $state(initialWorkoutType);
	let editMode = $state(false);

	const workoutTypes: { value: WorkoutType; label: string }[] = [
		{ value: 'easy', label: 'Easy' },
		{ value: 'tempo', label: 'Tempo' },
		{ value: 'intervals', label: 'Intervals' },
		{ value: 'long_run', label: 'Long Run' },
		{ value: 'race', label: 'Race' }
	];

	const getWorkoutTypeLabel = (value: WorkoutType | null): string => {
		if (value === null) return 'Not set';
		const type = workoutTypes.find((t) => t.value === value);
		return type?.label ?? value;
	};

	const getWorkoutTypeColor = (value: string | null): string => {
		if (value === null) return 'badge-ghost';
		switch (value) {
			case 'easy':
				return 'badge-success';
			case 'tempo':
				return 'badge-warning';
			case 'intervals':
				return 'badge-error';
			case 'long_run':
				return 'badge-info';
			case 'race':
				return 'badge-secondary';
			default:
				return 'badge-neutral';
		}
	};

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
			{#each workoutTypes as type}
				<button
					class={`btn btn-sm ${workoutType === type.value ? 'btn-active' : 'btn-ghost'}`}
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
	<div class="flex items-center gap-2">
		<div class="text-sm font-medium">Workout Type:</div>
		<span class={`badge ${getWorkoutTypeColor(workoutType)}`}>
			{getWorkoutTypeLabel(workoutType)}
		</span>
		<button class="btn btn-ghost btn-xs" onclick={() => (editMode = true)}>✏️ Edit</button>
	</div>
{/if}
