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

<div class="flex flex-wrap gap-6 sm:gap-8">
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
