<script lang="ts">
	import { isSome, type Option, none, some } from '$lib/Options';
	import { WORKOUT_TYPE_VALUES, workoutTypeDisplay, type WorkoutType } from '$lib/workout-type';

	let {
		workoutTypes = $bindable()
	}: {
		workoutTypes: Option<WorkoutType[]>;
	} = $props();
	let editing = $state(false);

	const addType = (wType: WorkoutType) => {
		if (isSome(workoutTypes)) {
			workoutTypes = some([...workoutTypes.value, wType]);
		}
	};
	const removeType = (wType: WorkoutType) => {
		if (isSome(workoutTypes)) {
			workoutTypes = some(workoutTypes.value.filter((r) => r !== wType));
		}
	};
</script>

{#if isSome(workoutTypes)}
	<div class="flex flex-col gap-1 rounded border border-primary/40 p-1.5 shadow">
		<div class="flex flex-row items-center justify-between">
			<div class="flex-1 text-wrap break-words">
				Workout types: {workoutTypes.value.toSorted().map(workoutTypeDisplay).join(', ')}
			</div>
			<div class="join shrink-0">
				<button class="btn join-item btn-xs" onclick={() => (editing = !editing)}>
					<img src="/icons/edit.svg" alt="Pen editing icon" class="inline h-5 w-5" />
				</button>
				<button class="btn join-item btn-xs" onclick={() => (workoutTypes = none())}>
					<img src="/icons/delete.svg" alt="Bin delete icon" class="inline h-5 w-5" />
				</button>
			</div>
		</div>
		{#if editing}
			<div class="divider my-0.5 px-2"></div>
			<div class="grid grid-cols-2 gap-2 px-1">
				{#each WORKOUT_TYPE_VALUES as workout (workout)}
					<label class="label cursor-pointer justify-start gap-2">
						<input
							type="checkbox"
							class="checkbox checkbox-xs"
							value={workout}
							checked={workoutTypes.value.includes(workout)}
							onchange={(e) => {
								if (e.currentTarget.checked) {
									addType(workout);
								} else {
									removeType(workout);
								}
							}}
						/>
						<span class="label-text text-xs">{workoutTypeDisplay(workout)}</span>
					</label>
				{/each}
			</div>
		{/if}
	</div>
{/if}
