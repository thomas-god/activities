<script lang="ts">
	import { timeseriesAvg, timeseriesMaximum, timeseriesQuarticAvg } from '$lib/timeseries';
	import Statistic from '../molecules/Statistic.svelte';
	import type { ActivityDetails } from '../routes/activity/[activity_id]/proxy+page';

	let { activity }: { activity: ActivityDetails } = $props();

	let statistics = $derived(new Map(Object.entries(activity.statistics)));

	let calories = $derived(statistics.get('Calories'));
	let distance = $derived.by(() => {
		const value = statistics.get('Distance');
		if (value === undefined) {
			return value;
		}
		return value / 1000;
	});
	let elevation = $derived(statistics.get('Elevation'));
	let avgHeartRate = $derived(timeseriesAvg(activity.timeseries.metrics, 'HeartRate'));
	let maxHeartRate = $derived(timeseriesMaximum(activity.timeseries.metrics, 'HeartRate'));
	let averageSpeed = $derived(timeseriesAvg(activity.timeseries.metrics, 'Speed'));
	let averagePower = $derived(timeseriesAvg(activity.timeseries.metrics, 'Power'));
	let weightedAveragePower = $derived(timeseriesQuarticAvg(activity.timeseries.metrics, 'Power'));
</script>

<div class="pb-1 text-lg">Statistics</div>

<div class="flex flex-row flex-wrap gap-2">
	<Statistic title="Distance" value={distance} unit="km" round={3} />
	<Statistic title="Calories" value={calories} unit="kcal" />
	<Statistic title="Elevation gained" value={elevation} unit="m" />
	<Statistic title="Average heart rate" value={avgHeartRate} unit="bpm" />
	<Statistic title="Maximum heart rate" value={maxHeartRate} unit="bpm" />
	<Statistic title="Average speed" value={averageSpeed} unit="km/h" round={2} />
	<Statistic title="Average power" value={averagePower} unit="W" />
	<Statistic title="Weighted average power" value={weightedAveragePower} unit="W" />
</div>
