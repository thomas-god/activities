<script lang="ts">
	import type { ActivityList } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { getSportCategoryIcon, sportDisplay, type SportCategory } from '$lib/sport';

	let {
		activityList,
		currentMonth = dayjs().startOf('month'),
		onMonthChange
	}: {
		activityList: ActivityList;
		currentMonth?: ReturnType<typeof dayjs>;
		onMonthChange?: (month: ReturnType<typeof dayjs>) => void;
	} = $props();

	// Track selected day for mobile view
	let selectedDay = $state<string | null>(null);

	// Compute activities grouped by date
	let activitiesByDate = $derived.by(() => {
		const grouped = new Map<string, typeof activityList>();

		activityList.forEach((activity) => {
			const dateKey = dayjs(activity.start_time).format('YYYY-MM-DD');
			if (!grouped.has(dateKey)) {
				grouped.set(dateKey, []);
			}
			grouped.get(dateKey)!.push(activity);
		});

		return grouped;
	});

	// Generate calendar days for current month
	let calendarDays = $derived.by(() => {
		const startOfMonth = currentMonth.startOf('month');
		const endOfMonth = currentMonth.endOf('month');
		const startDay = startOfMonth.startOf('isoWeek'); // Start from Monday
		const endDay = endOfMonth.endOf('isoWeek');

		const days = [];
		let current = startDay;

		while (current.isBefore(endDay) || current.isSame(endDay, 'day')) {
			const dateKey = current.format('YYYY-MM-DD');
			const activities = activitiesByDate.get(dateKey) || [];
			const isCurrentMonth = current.month() === currentMonth.month();
			const isToday = current.isSame(dayjs(), 'day');

			days.push({
				date: current,
				dateKey,
				day: current.date(),
				activities,
				isCurrentMonth,
				isToday
			});

			current = current.add(1, 'day');
		}

		return days;
	});

	const previousMonth = () => {
		onMonthChange?.(currentMonth.subtract(1, 'month'));
	};

	const nextMonth = () => {
		onMonthChange?.(currentMonth.add(1, 'month'));
	};

	const goToToday = () => {
		onMonthChange?.(dayjs().startOf('month'));
	};

	const handleMonthInput = (event: Event) => {
		const input = event.target as HTMLInputElement;
		const [year, month] = input.value.split('-').map(Number);
		onMonthChange?.(
			dayjs()
				.year(year)
				.month(month - 1)
				.startOf('month')
		);
	};

	// Format current month for the input value (YYYY-MM)
	const monthInputValue = $derived(currentMonth.format('YYYY-MM'));

	const formatDuration = (seconds: number): string => {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		if (hours > 0) {
			return `${hours}h${minutes > 0 ? minutes + 'm' : ''}`;
		}
		return `${minutes}m`;
	};

	const activitySportCategoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};

	const handleDayClick = (dateKey: string) => {
		selectedDay = selectedDay === dateKey ? null : dateKey;
	};

	// Get activities for selected day
	const selectedDayActivities = $derived(
		selectedDay ? activitiesByDate.get(selectedDay) || [] : []
	);
</script>

<div class="rounded-box bg-base-100 shadow-md">
	<!-- Calendar Header -->
	<div class="flex items-center justify-between border-b border-base-300 p-2 sm:p-4">
		<div class="flex items-center gap-1 sm:gap-2">
			<button onclick={previousMonth} class="btn btn-square btn-ghost btn-xs sm:btn-sm">
				<span class="text-lg">‹</span>
			</button>
			<button onclick={nextMonth} class="btn btn-square btn-ghost btn-xs sm:btn-sm">
				<span class="text-lg">›</span>
			</button>
		</div>

		<!-- Month/Year Picker -->
		<input
			type="month"
			value={monthInputValue}
			oninput={handleMonthInput}
			class="input input-sm input-ghost text-center text-base font-semibold sm:input-md sm:text-lg"
		/>

		<button onclick={goToToday} class="btn btn-ghost btn-xs sm:btn-sm">Today</button>
	</div>

	<!-- Calendar Grid -->
	<div class="p-2 sm:p-4">
		<!-- Day Headers -->
		<div class="mb-2 grid grid-cols-7 gap-1 sm:gap-2">
			{#each ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as dayName}
				<div class="text-center text-xs font-medium opacity-60 sm:text-sm">{dayName}</div>
			{/each}
		</div>

		<!-- Calendar Days -->
		<div class="grid grid-cols-7 gap-1 sm:gap-2">
			{#each calendarDays as day}
				<div
					role="button"
					tabindex={day.isCurrentMonth ? 0 : -1}
					onclick={() => handleDayClick(day.dateKey)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							handleDayClick(day.dateKey);
						}
					}}
					class="min-h-16 cursor-pointer rounded-lg border p-1 transition-colors hover:bg-base-200 sm:min-h-24 sm:p-2 {day.isCurrentMonth
						? 'border-base-300 bg-base-100'
						: 'border-base-200 bg-base-200 opacity-40'} {day.isToday
						? 'ring-2 ring-primary'
						: ''} {selectedDay === day.dateKey ? 'ring-2 ring-secondary' : ''}"
				>
					<div class="mb-1 flex items-center justify-between">
						<span
							class="text-xs font-medium sm:text-sm {day.isToday
								? 'flex h-5 w-5 items-center justify-center rounded-full bg-primary text-primary-content sm:h-6 sm:w-6'
								: ''}"
						>
							{day.day}
						</span>
					</div>

					<!-- Activities for this day -->
					<!-- Mobile: Show only colored dots -->
					<div class="flex flex-wrap gap-0.5 sm:hidden">
						{#each day.activities.slice(0, 6) as activity}
							<a
								href={`/activity/${activity.id}`}
								class={`activity-dot h-2 w-2 rounded-full ${activitySportCategoryClass(activity.sport_category)}`}
								title={activity.name || sportDisplay(activity.sport)}
								aria-label={activity.name || sportDisplay(activity.sport)}
							></a>
						{/each}
					</div>

					<!-- Desktop: Show activity details -->
					<div class="hidden flex-col gap-1 sm:flex">
						{#each day.activities.slice(0, 3) as activity}
							<a
								href={`/activity/${activity.id}`}
								class={`activity-details flex items-center gap-1 rounded-md bg-base-200 px-2 py-1 text-xs hover:bg-base-300 ${activitySportCategoryClass(activity.sport_category)}`}
							>
								<span class="text-base leading-none">
									{getSportCategoryIcon(activity.sport_category)}
								</span>
								<span class="flex-1 truncate">
									{activity.name || sportDisplay(activity.sport)}
								</span>
								<span class="text-xs opacity-60">
									{formatDuration(activity.duration)}
								</span>
							</a>
						{/each}
						{#if day.activities.length > 3}
							<div class="px-2 text-xs opacity-60">+{day.activities.length - 3} more</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>

		<!-- Mobile: Selected Day Activities Card -->
		{#if selectedDay}
			<div class="mt-4 rounded-lg bg-base-200 p-4 sm:hidden">
				<div class="mb-3 flex items-center justify-between">
					<h3 class="text-base font-semibold">
						{dayjs(selectedDay).format('dddd, MMMM D')}
					</h3>
					<button
						onclick={() => (selectedDay = null)}
						class="btn btn-circle btn-ghost btn-sm"
						aria-label="Close"
					>
						✕
					</button>
				</div>
				{#if selectedDayActivities.length > 0}
					<div class="flex flex-col gap-2">
						{#each selectedDayActivities as activity}
							<a
								href={`/activity/${activity.id}`}
								class={`activity-card flex items-center gap-3 rounded-lg bg-base-100 p-3 transition-colors hover:bg-base-300 ${activitySportCategoryClass(activity.sport_category)}`}
							>
								<span class="text-2xl leading-none">
									{getSportCategoryIcon(activity.sport_category)}
								</span>
								<div class="flex-1">
									<div class="font-medium">
										{activity.name || sportDisplay(activity.sport)}
									</div>
									<div class="text-xs opacity-60">
										{dayjs(activity.start_time).format('HH:mm')} • {formatDuration(
											activity.duration
										)}
									</div>
								</div>
								<span class="text-lg opacity-40">›</span>
							</a>
						{/each}
					</div>
				{:else}
					<div class="py-4 text-center opacity-60">No activities on this day</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.activity-details {
		border-left-width: 4px;

		&.running {
			border-left-color: var(--color-running);
		}
		&.cycling {
			border-left-color: var(--color-cycling);
		}
		&.other {
			border-left-color: var(--color-other);
		}
	}

	.activity-dot {
		&.running {
			background-color: var(--color-running);
		}
		&.cycling {
			background-color: var(--color-cycling);
		}
		&.other {
			background-color: var(--color-other);
		}
	}

	.activity-card {
		border-left-width: 4px;

		&.running {
			border-left-color: var(--color-running);
		}
		&.cycling {
			border-left-color: var(--color-cycling);
		}
		&.other {
			border-left-color: var(--color-other);
		}
	}
</style>
