<script lang="ts">
	import type { ActivityWithTimeseries } from '$lib/api';
	import { formatDuration } from '$lib/duration';
	import { paceToString } from '$lib/speed';

	let { activity }: { activity: ActivityWithTimeseries } = $props();

	let metrics = $derived(new Map(Object.entries(activity.metrics)));

	let calories = $derived(metrics.get('Calories'));
	let distance = $derived.by(() => {
		const value = metrics.get('Distance');
		if (value === undefined) {
			return value;
		}
		return value / 1000;
	});
	let duration = $derived(metrics.get('ActiveDuration'));
	let elevation = $derived(metrics.get('Elevation'));
	let avgHeartRate = $derived(metrics.get('AvgHeartRate'));
	let maxHeartRate = $derived(metrics.get('MaxHeartRate'));
	let averageSpeed = $derived(metrics.get('AvgSpeed'));
	let averagePace = $derived(metrics.get('AvgPace'));
	let averagePower = $derived(metrics.get('AvgPower'));
	let normalizedPower = $derived(metrics.get('NormalizedPower'));

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
				icon: 'duration.svg',
				label: 'Duration',
				value: formatDuration(duration),
				unit: ''
			});
		}

		if (distance !== undefined) {
			rows.push({
				icon: 'distance.svg',
				label: 'Distance',
				value: `${distance.toFixed(3)}`,
				unit: 'km'
			});
		}

		// Speed
		if (averageSpeed !== undefined) {
			if (activity.sport_category === 'Running') {
				rows.push({
					icon: 'pace.svg',
					label: 'Pace',
					value: paceToString((averagePace! * 1000) / 60),
					unit: '/km',
					legend: 'avg'
				});
			} else {
				rows.push({
					icon: 'pace.svg',
					label: 'Speed',
					value: `${(averageSpeed * 3.6).toFixed(2)}`,
					unit: 'km/h',
					legend: 'avg'
				});
			}
		}

		if (elevation !== undefined) {
			rows.push({
				icon: 'elevation.svg',
				label: 'Elevation',
				value: `${elevation.toFixed(0)}`,
				unit: 'm'
			});
		}

		if (avgHeartRate !== undefined && maxHeartRate !== undefined) {
			rows.push({
				icon: 'cardio.svg',
				label: 'Heart rate',
				value: `${avgHeartRate.toFixed(0)} / ${maxHeartRate.toFixed(0)}`,
				unit: 'bpm',
				legend: 'avg / max'
			});
		}

		if (calories !== undefined) {
			rows.push({
				icon: 'calories.svg',
				label: 'Calories',
				value: `${calories.toFixed(0)}`,
				unit: 'kcal'
			});
		}

		if (averagePower !== undefined && normalizedPower !== undefined) {
			rows.push({
				icon: 'power.svg',
				label: 'Power',
				value: `${averagePower.toFixed(0)} / ${normalizedPower.toFixed(0)}`,
				unit: 'W',
				legend: 'avg / normalized'
			});
		}

		return rows;
	});
</script>

<div class="@container">
	<div class="hidden @lg:grid @lg:grid-cols-2 @min-[52rem]:grid-cols-3">
		{#each statRows as row}
			<div class="flex items-center gap-3 border-b border-base-300 p-4 hover:bg-base-200">
				<img src={`/icons/${row.icon}`} class="h-6 w-6" alt={`${row.label} icon`} />
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
