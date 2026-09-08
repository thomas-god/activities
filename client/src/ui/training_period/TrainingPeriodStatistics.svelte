<script lang="ts">
	import type { TrainingPeriodDetails } from '$lib/api';
	import { formatDurationHoursMinutes } from '$lib/duration';
	import { formatDistance, formatElevation, periodActivitiesSummary } from '$lib/trainingPeriod';

	interface Props {
		period: TrainingPeriodDetails;
	}

	let { period }: Props = $props();

	const summary = $derived(periodActivitiesSummary(period.activities));
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
