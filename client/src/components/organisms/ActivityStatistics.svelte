<script lang="ts">
	import type { ActivityDetails } from '$lib/api';
	import { formatDuration } from '$lib/duration';
	import { paceToString, speedToPace } from '$lib/speed';
	import { timeseriesAvg, timeseriesMaximum, timeseriesQuarticAvg } from '$lib/timeseries';

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
	let averagePace = $derived(averageSpeed === undefined ? undefined : speedToPace(averageSpeed));
	let averagePower = $derived(timeseriesAvg(activity.timeseries.metrics, 'Power'));
	let weightedAveragePower = $derived(timeseriesQuarticAvg(activity.timeseries.metrics, 'Power'));

	type StatRow = {
		icon: string;
		label: string;
		value: string | undefined;
		unit: string;
		legend?: string;
	};

	let statRows = $derived.by<StatRow[]>(() => {
		const rows: StatRow[] = [];

		if (duration !== undefined) {
			rows.push({
				icon: '⌛',
				label: 'Duration',
				value: formatDuration(duration),
				unit: ''
			});
		}

		if (distance !== undefined) {
			rows.push({
				icon: '📏',
				label: 'Distance',
				value: `${distance.toFixed(3)}`,
				unit: 'km'
			});
		}

		// Speed
		if (averageSpeed !== undefined) {
			if (activity.sport_category === 'Running') {
				rows.push({
					icon: '⚡',
					label: 'Pace',
					value: paceToString(averagePace!),
					unit: '/km',
					legend: 'avg'
				});
			} else {
				rows.push({
					icon: '⚡',
					label: 'Speed',
					value: `${averageSpeed.toFixed(2)}`,
					unit: 'km/h',
					legend: 'avg'
				});
			}
		}

		if (elevation !== undefined) {
			rows.push({
				icon: '⛰️',
				label: 'Elevation',
				value: `${elevation.toFixed(0)}`,
				unit: 'm'
			});
		}

		if (avgHeartRate !== undefined && maxHeartRate !== undefined) {
			rows.push({
				icon: '❤️',
				label: 'Heart rate',
				value: `${avgHeartRate.toFixed(0)} / ${maxHeartRate.toFixed(0)}`,
				unit: 'bpm',
				legend: 'avg / max'
			});
		}

		if (calories !== undefined) {
			rows.push({
				icon: '🔥',
				label: 'Calories',
				value: `${calories.toFixed(0)}`,
				unit: 'kcal'
			});
		}

		if (averagePower !== undefined && weightedAveragePower !== undefined) {
			rows.push({
				icon: '⚙️',
				label: 'Power',
				value: `${averagePower.toFixed(0)} / ${weightedAveragePower.toFixed(0)}`,
				unit: 'W',
				legend: 'avg / weighted'
			});
		}

		return rows;
	});
</script>

<div class="@container">
	<div class="hidden @lg:grid @lg:grid-cols-2 @min-[52rem]:grid-cols-3">
		{#each statRows as row}
			<div class="flex items-center gap-3 border-b border-base-300 p-4 hover:bg-base-200">
				<div class="text-2xl">{row.icon}</div>
				<div class=" flex-1 font-medium">{row.label}</div>
				<div class="text-right {row.legend ? '' : 'self-center'}">
					<div class="text-lg font-semibold">{row.value || '-'} {row.unit}</div>
					{#if row.legend}
						<div class="text-xs opacity-60">
							{row.legend}
						</div>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<div class="grid grid-cols-3 gap-0.5">
		{#each statRows as row}
			<div class="flex h-16 flex-col gap-0 pb-3 text-left @lg:hidden">
				<div class="text-lg font-semibold">
					{row.value || '-'}
					<span class="text-xs font-medium">
						{row.unit}
					</span>
				</div>
				<div class="text-xs">
					<span class="opacity-60">
						<!-- {row.icon} -->
						{row.label}

						{#if row.legend}
							<span class="ml-1">({row.legend})</span>
						{/if}
					</span>
				</div>
			</div>
		{/each}
	</div>
</div>
