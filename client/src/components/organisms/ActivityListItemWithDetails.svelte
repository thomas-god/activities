<script lang="ts">
	import { formatRelativeDuration, dayjs, formatDurationCompactWithUnits } from '$lib/duration';
	import { getSportCategoryIcon, sportDisplay, type SportCategory } from '$lib/sport';
	import type { Activity, ActivityListSummaryItems } from '$lib/api';
	import { getWorkoutTypeClass, getWorkoutTypeLabel } from '$lib/workout-type';
	import { getRpeClass } from '$lib/rpe';

	let {
		activity,
		onClick,
		listFormat,
		isSelected = false
	}: {
		activity: Activity;
		onClick?: () => void;
		isSelected?: boolean;
		listFormat: ActivityListSummaryItems;
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

	const formatMetric = (
		name: string,
		metric: Activity['metrics'][string]
	): { value: string; unit: string } => {
		if (metric.unit === 's') {
			return { value: formatDurationCompactWithUnits(metric.value), unit: '' };
		}
		if (name === 'Distance' && metric.unit === 'm') {
			return { value: (metric.value / 1000).toFixed(1), unit: 'km' };
		}
		return { value: metric.value.toFixed(0), unit: metric.unit };
	};

	const handleClick = (event: MouseEvent) => {
		if (onClick) {
			event.preventDefault();
			onClick();
		}
	};
	let selectedClass = $derived(isSelected ? 'selected' : '');
</script>

<div
	class={`
		icon_container col-start-1
		${categoryClass(activity.sport_category)}
		${selectedClass}`}
>
	<div class={`icon ${categoryClass(activity.sport_category)}`}>
		<img
			src={`/icons/${getSportCategoryIcon(activity.sport_category)}`}
			class="h-6 w-6"
			alt="Sport icon"
		/>
	</div>
</div>

<div
	class={`flex flex-col h-full w-full justify-center col-start-2 sticky-left bg-base-100 ${categoryClass(activity.sport_category)} ${selectedClass}`}
>
	<div class="mb-1 font-semibold">
		<a href={`/activity/${activity.id}`} onclick={handleClick} class="hover:link">
			{title}
		</a>
	</div>
	<div class="text-xs font-light">
		{formatRelativeDuration(dayjs(activity.start_time), dayjs())} · {dayjs(
			activity.start_time
		).format('MMM D, YYYY')}
	</div>
</div>

{#each listFormat as row, idx}
	<div class="w-full h-full flex flex-col justify-center items-center">
		{#if row.type === 'rpe' && activity.rpe}
			<span class={`badge badge-sm inline ${getRpeClass(activity.rpe)}`}>
				RPE {activity.rpe}
			</span>
		{:else if row.type === 'workoutType' && activity.workout_type}
			<span class={`badge badge-sm inline ${getWorkoutTypeClass(activity.workout_type)}`}>
				{getWorkoutTypeLabel(activity.workout_type)}
			</span>
		{:else if row.type === 'metric'}
			{@const metric = activity.metrics[row.value]}
			{#if metric !== undefined}
				{@const formattedMetric = formatMetric(row.value, metric)}
				<span>
					<span class="font-semibold">
						{formattedMetric.value}
					</span>
					<span class="text-xs font-light">
						{formattedMetric.unit}
					</span>
				</span>
			{:else}
				-
			{/if}
		{/if}
	</div>
{/each}

{#if activity.feedback}
	<div
		class="sticky-left mx-3 my-1 box-border flex flex-row items-start gap-1 bg-orange-200/10 py-2 pl-2 text-sm whitespace-pre-wrap text-gray-600 italic"
		style:grid-column={`1 / span ${listFormat.length + 2}`}
	>
		<div class="shrink-0"><img src="/icons/note.svg" class="h-5 w-5" alt="Memo icon" /></div>
		<div class="feedback">
			{activity.feedback}
		</div>
	</div>
{/if}

<style>
	.sticky-left {
		position: sticky;
		left: 0;
	}

	.icon_container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding-block: calc(var(--spacing) * 2);
		padding-right: calc(var(--spacing) * 2);
		padding-left: calc(var(--spacing) * 2);
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 0px;
	}

	@media (min-width: 700px) {
		.icon_container.selected {
			border-left-width: 6px;
		}
		.selected {
			background: #e6eef5;
		}
	}

	.icon_container.cycling {
		border-color: var(--color-cycling);
	}

	.icon_container.running {
		border-color: var(--color-running);
	}

	.icon_container.other {
		border-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
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

	.feedback {
		max-width: min(75vw, 500px);
	}
</style>
