<script lang="ts">
	import type { ActivityWithTimeseries } from '$lib/api/activities';
	import { dayjs } from '$lib/duration';
	import { paceToString, speedToPace } from '$lib/speed';

	export type LapMetric = 'distance' | 'speed' | 'power' | 'heartRate' | 'pace';

	interface Props {
		activity: ActivityWithTimeseries;
		selectedLap: ActivityWithTimeseries['timeseries']['laps'][number] | null;
		onLapSelectedCallback: (lap: ActivityWithTimeseries['timeseries']['laps'][number]) => void;
	}

	let { activity, selectedLap = $bindable(), onLapSelectedCallback }: Props = $props();

	let laps = $derived(activity.timeseries.laps);
	let metrics: LapMetric[] = $derived.by(() => {
		switch (activity.sport_category) {
			case 'Running':
				return ['distance', 'pace', 'heartRate'];
			case 'Cycling':
				return ['distance', 'power', 'heartRate'];
			case 'Rowing':
				return ['distance', 'speed', 'power', 'heartRate'];
			case 'Swimming':
				return ['distance', 'speed'];
			case 'Ski':
				return ['distance', 'speed', 'heartRate'];
			case 'Walking':
				return ['distance', 'speed', 'heartRate'];
			case 'Cardio':
				return ['heartRate'];
			case 'Climbing':
				return ['heartRate'];
			case 'TeamSports':
			case 'Racket':
			case 'WaterSports':
				return ['distance', 'speed', 'heartRate'];
			default:
				return ['heartRate'];
		}
	});
	const colTemplate = $derived(`auto auto ${metrics.map(() => '1fr').join(' ')}`);
	let hoveredLapIndex = $state<number | null>(null);

	function formatDuration(seconds: number): string {
		const d = dayjs.duration(seconds, 'seconds');
		const hours = Math.floor(d.asHours());
		const minutes = d.minutes();
		const secs = d.seconds();

		if (hours > 0) {
			return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
		}
		return `${minutes}:${secs.toString().padStart(2, '0')}`;
	}

	function calculateLapMetric(
		lapStart: number,
		lapEnd: number,
		metric: LapMetric
	): string | undefined {
		const startIndex = activity.timeseries.time.findIndex((t) => t >= lapStart);
		const endIndex = activity.timeseries.time.findIndex((t) => t >= lapEnd);

		if (startIndex === -1 || endIndex === -1) {
			return undefined;
		}

		switch (metric) {
			case 'distance': {
				const distanceMetric = activity.timeseries.metrics['Distance'];
				if (!distanceMetric) return undefined;

				const startDistance = distanceMetric.values[startIndex] ?? 0;
				const endDistance = distanceMetric.values[endIndex] ?? 0;
				const lapDistance = endDistance - startDistance;

				return lapDistance > 0 ? `${lapDistance.toFixed(2)} km` : undefined;
			}

			case 'speed': {
				const speedMetric = activity.timeseries.metrics['Speed'];
				if (!speedMetric) return undefined;

				const lapValues = speedMetric.values.slice(startIndex, endIndex + 1);
				const validValues = lapValues.filter((v): v is number => v !== null);

				if (validValues.length === 0) return undefined;

				const avg = validValues.reduce((sum, v) => sum + v, 0) / validValues.length;
				return `${avg.toFixed(2)} km/h`;
			}

			case 'pace': {
				const speedMetric = activity.timeseries.metrics['Speed'];
				if (!speedMetric) return undefined;

				const lapValues = speedMetric.values.slice(startIndex, endIndex + 1);
				const validValues = lapValues.filter((v): v is number => v !== null);

				if (validValues.length === 0) return undefined;

				const avg = validValues.reduce((sum, v) => sum + v, 0) / validValues.length;
				return paceToString(speedToPace(avg), true) + ' /km';
			}

			case 'power': {
				const powerMetric = activity.timeseries.metrics['Power'];
				if (!powerMetric) return undefined;

				const lapValues = powerMetric.values.slice(startIndex, endIndex + 1);
				const validValues = lapValues.filter((v): v is number => v !== null);

				if (validValues.length === 0) return undefined;

				const avg = validValues.reduce((sum, v) => sum + v, 0) / validValues.length;
				return `${avg.toFixed(0)} W`;
			}

			case 'heartRate': {
				const hrMetric = activity.timeseries.metrics['HeartRate'];
				if (!hrMetric) return undefined;

				const lapValues = hrMetric.values.slice(startIndex, endIndex + 1);
				const validValues = lapValues.filter((v): v is number => v !== null);

				if (validValues.length === 0) return undefined;

				const avg = validValues.reduce((sum, v) => sum + v, 0) / validValues.length;
				return `${avg.toFixed(0)} bpm`;
			}

			default:
				return undefined;
		}
	}

	function getMetricLabel(metric: LapMetric): string {
		switch (metric) {
			case 'distance':
				return 'Distance';
			case 'speed':
				return 'Avg Speed';
			case 'power':
				return 'Avg Power';
			case 'heartRate':
				return 'Avg HR';
			case 'pace':
				return 'Avg pace';
		}
	}
</script>

{#if laps.length > 0}
	<div
		role="grid"
		tabindex="-1"
		class="grid overflow-x-scroll text-sm"
		style="grid-template-columns: {colTemplate}"
		onmouseleave={() => {
			selectedLap = null;
			hoveredLapIndex = null;
		}}
	>
		<div role="row" class="contents">
			<div role="columnheader" class="border-b border-base-300 px-3 py-2 font-semibold">Lap</div>
			<div role="columnheader" class="border-b border-base-300 px-3 py-2 font-semibold">
				Duration
			</div>
			{#each metrics as metric}
				<div role="columnheader" class="border-b border-base-300 px-3 py-2 font-semibold">
					{getMetricLabel(metric)}
				</div>
			{/each}
		</div>

		{#each laps as lap, index}
			{@const isHovered = hoveredLapIndex === index}
			{@const isOdd = index % 2 === 1}
			<div
				role="row"
				tabindex="0"
				class="contents cursor-pointer"
				onmouseenter={() => {
					hoveredLapIndex = index;
					selectedLap = lap;
				}}
				onclick={() => onLapSelectedCallback(lap)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') onLapSelectedCallback(lap);
				}}
			>
				<div
					role="gridcell"
					class="px-3 py-2"
					class:bg-base-200={isOdd && !isHovered}
					class:bg-base-300={isHovered}
				>
					{index + 1}
				</div>
				<div
					role="gridcell"
					class="px-3 py-2"
					class:bg-base-200={isOdd && !isHovered}
					class:bg-base-300={isHovered}
				>
					{formatDuration(lap.end - lap.start)}
				</div>
				{#each metrics as metric}
					<div
						role="gridcell"
						class="px-3 py-2"
						class:bg-base-200={isOdd && !isHovered}
						class:bg-base-300={isHovered}
					>
						{calculateLapMetric(lap.start, lap.end, metric) ?? '-'}
					</div>
				{/each}
			</div>
		{/each}
	</div>
{/if}
