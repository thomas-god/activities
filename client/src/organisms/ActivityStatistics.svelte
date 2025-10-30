<script lang="ts">
	import { formatDuration } from '$lib/duration';
	import { timeseriesAvg, timeseriesMaximum, timeseriesQuarticAvg } from '$lib/timeseries';
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
	let duration = $derived(statistics.get('Duration'));
	let elevation = $derived(statistics.get('Elevation'));
	let avgHeartRate = $derived(timeseriesAvg(activity.timeseries.metrics, 'HeartRate'));
	let maxHeartRate = $derived(timeseriesMaximum(activity.timeseries.metrics, 'HeartRate'));
	let averageSpeed = $derived(timeseriesAvg(activity.timeseries.metrics, 'Speed'));
	let averagePower = $derived(timeseriesAvg(activity.timeseries.metrics, 'Power'));
	let weightedAveragePower = $derived(timeseriesQuarticAvg(activity.timeseries.metrics, 'Power'));

	type StatRow = {
		icon: string;
		label: string;
		value: string | undefined;
		subvalue?: string;
	};

	let statRows = $derived.by<StatRow[]>(() => {
		const rows: StatRow[] = [];

		if (duration !== undefined) {
			rows.push({
				icon: '⌛',
				label: 'Duration',
				value: formatDuration(duration)
			});
		}

		if (distance !== undefined) {
			rows.push({
				icon: '📏',
				label: 'Distance',
				value: `${distance.toFixed(3)} km`
			});
		}

		// Speed
		if (averageSpeed !== undefined) {
			rows.push({
				icon: '⚡',
				label: 'Speed',
				value: `${averageSpeed.toFixed(2)} km/h`,
				subvalue: 'avg'
			});
		}

		if (elevation !== undefined) {
			rows.push({
				icon: '⛰️',
				label: 'Elevation',
				value: `${elevation.toFixed(0)} m`
			});
		}

		if (calories !== undefined) {
			rows.push({
				icon: '🔥',
				label: 'Calories',
				value: `${calories.toFixed(0)} kcal`
			});
		}

		if (avgHeartRate !== undefined && maxHeartRate !== undefined) {
			rows.push({
				icon: '❤️',
				label: 'Heart rate',
				value: `${avgHeartRate.toFixed(0)} / ${maxHeartRate.toFixed(0)} bpm`,
				subvalue: 'avg / max'
			});
		}

		if (averagePower !== undefined && weightedAveragePower !== undefined) {
			rows.push({
				icon: '⚙️',
				label: 'Power',
				value: `${averagePower.toFixed(0)} / ${weightedAveragePower.toFixed(0)} W`,
				subvalue: 'avg / weighted'
			});
		}

		return rows;
	});
</script>

<details class="collapse-arrow collapse rounded-box border border-base-300 bg-base-100 shadow" open>
	<summary class="collapse-title text-lg font-semibold">Statistics</summary>
	<div class="@container collapse-content">
		<div class="grid grid-cols-1 @xl:grid-cols-2 @min-[52rem]:grid-cols-3">
			{#each statRows as row}
				<div class="flex items-center gap-3 border-b border-base-300 p-4 hover:bg-base-200">
					<div class="text-2xl">{row.icon}</div>
					<div class=" flex-1 font-medium">{row.label}</div>
					<div class="text-right {row.subvalue ? '' : 'self-center'}">
						<div class="text-lg font-semibold">{row.value || '-'}</div>
						{#if row.subvalue}
							<div class="text-xs opacity-60">
								{row.subvalue}
							</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	</div>
</details>
