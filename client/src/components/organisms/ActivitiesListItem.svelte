<script lang="ts">
	import { formatDuration, formatRelativeDuration, dayjs } from '$lib/duration';
	import { getSportCategoryIcon, sportDisplay, type SportCategory } from '$lib/sport';
	import type { ActivityListItem } from '$lib/api';
	import { getWorkoutTypeClass, getWorkoutTypeLabel } from '$lib/workout-type';
	import { getRpeClass } from '$lib/rpe';

	let {
		activity,
		showNote = false
	}: {
		activity: ActivityListItem;
		showNote?: boolean;
	} = $props();

	let title = $derived(
		activity.name === null || activity.name === '' ? sportDisplay(activity.sport) : activity.name
	);

	const categoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};
</script>

<a
	href={`/activity/${activity.id}`}
	class={`item @container py-1 ${categoryClass(activity.sport_category)}`}
>
	<div class={`flex flex-1 items-center pl-2 ${categoryClass(activity.sport_category)}`}>
		<div class={`icon ${categoryClass(activity.sport_category)}`}>
			{getSportCategoryIcon(activity.sport_category)}
		</div>
		<div class="flex-1">
			<div class="flex flex-col">
				<div class="mb-1 font-semibold">{title}</div>
				<div class="text-xs font-light">
					{formatRelativeDuration(dayjs(activity.start_time), dayjs())} · {dayjs(
						activity.start_time
					).format('MMM D, YYYY')}
				</div>
			</div>
		</div>
		<div class="flex flex-row items-center justify-center gap-2">
			{#if activity.workout_type}
				<span
					class={`badge hidden badge-sm @md:inline ${getWorkoutTypeClass(activity.workout_type)}`}
				>
					{getWorkoutTypeLabel(activity.workout_type)}
				</span>
			{/if}
			{#if activity.rpe}
				<span class={`badge hidden badge-sm @md:inline ${getRpeClass(activity.rpe)}`}>
					RPE {activity.rpe}
				</span>
			{/if}
			<span class="font-semibold sm:text-lg">
				{formatDuration(activity.duration)}
			</span>
		</div>
	</div>
	{#if showNote && activity.feedback}
		<div class="my-1 pl-4 text-sm whitespace-pre-wrap text-gray-600 italic">
			📝 {activity.feedback}
		</div>
	{/if}
</a>

<style>
	.item:hover {
		background: #f7fafc;
	}

	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 0px;
	}

	.item.cycling {
		border-left-color: var(--color-cycling);
	}

	.item.running {
		border-left-color: var(--color-running);
	}

	.item.other {
		border-left-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
	}

	.icon.cycling {
		background: var(--color-cycling-background);
		color: var(--color-cycling);
	}

	.icon.running {
		background: var(--color-running-background);
		color: var(--color-running);
	}

	.icon.other {
		background: var(--color-other-background);
		color: var(--color-other);
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
</style>
