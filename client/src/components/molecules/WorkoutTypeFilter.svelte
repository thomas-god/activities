<script lang="ts">
	import { WORKOUT_TYPE_LABELS, type WorkoutType } from '$lib/workout-type';

	let {
		enabled = $bindable(false),
		selectedWorkoutTypes = $bindable([])
	}: {
		enabled?: boolean;
		selectedWorkoutTypes?: WorkoutType[];
	} = $props();
</script>

<div class="mb-2 font-semibold">
	<span class="pr-2"> Filter activities by workout type </span>
	<input type="checkbox" bind:checked={enabled} class="toggle toggle-sm" />
</div>
{#if enabled}
	<div class="grid grid-cols-2 gap-2">
		{#each WORKOUT_TYPE_LABELS as { value, label } (value)}
			<label class="label cursor-pointer justify-start gap-2">
				<input
					type="checkbox"
					class="checkbox checkbox-sm"
					{value}
					checked={selectedWorkoutTypes.includes(value)}
					onchange={(e) => {
						if (e.currentTarget.checked) {
							selectedWorkoutTypes = [...selectedWorkoutTypes, value];
						} else {
							selectedWorkoutTypes = selectedWorkoutTypes.filter((wt) => wt !== value);
						}
					}}
				/>
				<span class="label-text text-xs">{label}</span>
			</label>
		{/each}
	</div>
{/if}
