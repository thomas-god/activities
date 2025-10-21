<script lang="ts">
	import type { ActivityList } from '$lib/api';
	import { dayjs } from '$lib/duration';
	import { getSportCategoryIcon } from '$lib/sport';

	let { activityList }: { activityList: ActivityList } = $props();

	// State for current month view
	let currentMonth = $state(dayjs().startOf('month'));

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
		currentMonth = currentMonth.subtract(1, 'month');
	};

	const nextMonth = () => {
		currentMonth = currentMonth.add(1, 'month');
	};

	const goToToday = () => {
		currentMonth = dayjs().startOf('month');
	};

	const formatDuration = (seconds: number): string => {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		if (hours > 0) {
			return `${hours}h${minutes > 0 ? minutes + 'm' : ''}`;
		}
		return `${minutes}m`;
	};
</script>

<div class="rounded-box bg-base-100 shadow-md">
	<!-- Calendar Header -->
	<div class="border-base-300 flex items-center justify-between border-b p-4">
		<div class="flex items-center gap-2">
			<button onclick={previousMonth} class="btn btn-square btn-ghost btn-sm">
				<span class="text-lg">‹</span>
			</button>
			<button onclick={nextMonth} class="btn btn-square btn-ghost btn-sm">
				<span class="text-lg">›</span>
			</button>
		</div>
		<h2 class="text-lg font-semibold">{currentMonth.format('MMMM YYYY')}</h2>
		<button onclick={goToToday} class="btn btn-ghost btn-sm">Today</button>
	</div>

	<!-- Calendar Grid -->
	<div class="p-4">
		<!-- Day Headers -->
		<div class="mb-2 grid grid-cols-7 gap-2">
			{#each ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as dayName}
				<div class="text-center text-sm font-medium opacity-60">{dayName}</div>
			{/each}
		</div>

		<!-- Calendar Days -->
		<div class="grid grid-cols-7 gap-2">
			{#each calendarDays as day}
				<div
					class="hover:bg-base-200 min-h-24 rounded-lg border p-2 transition-colors {day.isCurrentMonth
						? 'border-base-300 bg-base-100'
						: 'border-base-200 bg-base-200 opacity-40'} {day.isToday ? 'ring-primary ring-2' : ''}"
				>
					<div class="mb-1 flex items-center justify-between">
						<span
							class="text-sm font-medium {day.isToday
								? 'bg-primary text-primary-content flex h-6 w-6 items-center justify-center rounded-full'
								: ''}"
						>
							{day.day}
						</span>
						{#if day.activities.length > 0}
							<span class="badge badge-xs badge-primary">{day.activities.length}</span>
						{/if}
					</div>

					<!-- Activities for this day -->
					<div class="flex flex-col gap-1">
						{#each day.activities.slice(0, 3) as activity}
							<a
								href={`/activity/${activity.id}`}
								class="bg-base-200 hover:bg-base-300 flex items-center gap-1 rounded-md px-2 py-1 text-xs"
							>
								<span class="text-base leading-none">
									{getSportCategoryIcon(activity.sport_category)}
								</span>
								<span class="flex-1 truncate">
									{activity.name || activity.sport}
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
	</div>
</div>
