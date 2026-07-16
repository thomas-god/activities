<script lang="ts">
	import { formatRelativeDuration, dayjs, formatDurationCompactWithUnits } from '$lib/duration';
	import { getSportCategoryIcon, sportDisplay, type SportCategory } from '$lib/sport';
	import type { Activity } from '$lib/api';
	import { some, none, type Option, isSome } from '$lib/Options';

	let {
		activity,
		onClick,
		isSelected = false
	}: {
		activity: Activity;
		onClick?: () => void;
		isSelected?: boolean;
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

	let duration: Option<string> = $derived.by(() => {
		const duration = activity.metrics['ActiveDuration'];
		if (duration === undefined) {
			return none();
		}
		return some(formatDurationCompactWithUnits(duration.value));
	});

	const handleClick = (event: MouseEvent) => {
		if (onClick) {
			event.preventDefault();
			onClick();
		}
	};
	let selectedClass = $derived(isSelected ? 'selected' : '');
</script>

<a
	href={`/activity/${activity.id}`}
	class={`
		item_container w-full
		${categoryClass(activity.sport_category)}
		${selectedClass}`}
	onclick={handleClick}
>
	<div class="flex-1 flex flex-col">
		<div class="flex flex-row items-center">
			<!-- Sport icon -->
			<div class={`icon ${categoryClass(activity.sport_category)}`}>
				<img
					src={`/icons/${getSportCategoryIcon(activity.sport_category)}`}
					class="h-6 w-6"
					alt="Sport icon"
				/>
			</div>

			<!-- Activity name -->
			<div class="flex-1 flex flex-col h-full pl-2">
				<div class="mb-1 font-semibold">{title}</div>
				<div class="text-xs font-light">
					{formatRelativeDuration(dayjs(activity.start_time), dayjs())} · {dayjs(
						activity.start_time
					).format('MMM D, YYYY')}
				</div>
			</div>

			<!-- Activity duration -->
			{#if isSome(duration)}
				<span class="font-semibold">
					{duration.value}
				</span>
			{/if}
		</div>

		{#if activity.feedback}
			<div
				class="sticky-left mx-3 my-1 box-border flex flex-row items-start gap-1 bg-orange-200/10 py-2 pl-2 text-sm whitespace-pre-wrap text-gray-600 italic"
			>
				<div class="shrink-0"><img src="/icons/note.svg" class="h-5 w-5" alt="Memo icon" /></div>
				<div class="feedback">
					{activity.feedback}
				</div>
			</div>
		{/if}
	</div>
</a>

<style>
	a {
		padding-left: 2px;
		padding-right: 2px;
	}

	.hovered {
		background: #f7fafc;
	}

	.sticky-left {
		position: sticky;
		left: 0;
	}

	.item_container {
		padding-block: calc(var(--spacing) * 2);
		padding-right: calc(var(--spacing) * 2);
		padding-left: calc(var(--spacing) * 2);
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 0px;
	}

	@media (min-width: 700px) {
		.item_container.selected {
			border-left-width: 6px;
		}
		.selected {
			background: #e6eef5;
		}
	}

	.item_container.cycling {
		border-color: var(--color-cycling);
	}

	.item_container.running {
		border-color: var(--color-running);
	}

	.item_container.other {
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
