<script lang="ts">
	import type { TrainingPeriodDetails } from '$lib/api';
	import { dayjs, formatDurationHoursMinutes } from '$lib/duration';

	interface Props {
		period: TrainingPeriodDetails;
	}

	let { period }: Props = $props();

	const summary = $derived.by(() => {
		const total = {
			count: period.activities.length,
			duration: 0,
			distance: 0,
			elevation: 0
		};

		for (const activity of period.activities) {
			total.duration += activity.duration ?? 0;
			total.distance += activity.distance ?? 0;
			total.elevation += activity.elevation ?? 0;
		}

		return total;
	});

	const formatPeriodDuration = (start: string, end: string | null): string => {
		const startDate = dayjs(start);
		const endDate = end ? dayjs(end) : dayjs();
		// Add 1 to include the last day (end date is inclusive)
		const days = endDate.diff(startDate, 'day') + 1;

		if (days === 1) return '1 day';
		if (days < 7) return `${days} days`;

		const weeks = Math.floor(days / 7);
		const remainingDays = days % 7;

		if (remainingDays === 0) {
			return weeks === 1 ? '1 week' : `${weeks} weeks`;
		}

		const weeksText = weeks === 1 ? '1 week' : `${weeks} weeks`;
		const daysText = remainingDays === 1 ? '1 day' : `${remainingDays} days`;
		return `${weeksText} ${daysText}`;
	};

	const formatDistance = (meters: number): string => {
		if (meters === 0) return '0 km';
		const km = meters / 1000;
		return `${Math.round(km).toLocaleString('fr-fr')} km`;
	};

	const formatElevation = (meters: number): string => {
		if (meters === 0) return '0 m';
		return `${Math.round(meters).toLocaleString('fr-fr')} m`;
	};
</script>

<div class="flex flex-row flex-wrap gap-6 rounded bg-base-200 p-4">
	<div class="flex flex-col">
		<div class="text-xs opacity-70">Period Duration</div>
		<div class="text-xl font-semibold">{formatPeriodDuration(period.start, period.end)}</div>
	</div>
	<div class="flex flex-col">
		<div class="text-xs opacity-70">Activities</div>
		<div class="text-xl font-semibold">{summary.count}</div>
	</div>
	<div class="flex flex-col">
		<div class="text-xs opacity-70">Activities duration</div>
		<div class="text-xl font-semibold">
			{formatDurationHoursMinutes(summary.duration)}
		</div>
	</div>
	<div class="flex flex-col">
		<div class="text-xs opacity-70">Total Distance</div>
		<div class="text-xl font-semibold">{formatDistance(summary.distance)}</div>
	</div>
	<div class="flex flex-col">
		<div class="text-xs opacity-70">Total Elevation</div>
		<div class="text-xl font-semibold">{formatElevation(summary.elevation)}</div>
	</div>
</div>
