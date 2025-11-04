<script lang="ts">
	import type { ActivityDetails } from '$lib/api/activities';
	import { dayjs } from '$lib/duration';
	import { paceToString, speedToPace } from '$lib/speed';

	export type LapMetric = 'distance' | 'speed' | 'power' | 'heartRate' | 'pace';

	interface Props {
		activity: ActivityDetails;
	}

	let { activity }: Props = $props();

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
	const showMetrics = $derived(metrics.length > 0);

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
				return paceToString(speedToPace(avg), true);
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
	<details
		class="collapse-arrow collapse rounded-box border border-base-300 bg-base-100 shadow"
		open
	>
		<summary class="collapse-title text-lg font-semibold">Laps</summary>
		<div class="collapse-content">
			<div class="overflow-x-auto">
				<table class="table table-sm">
					<thead>
						<tr>
							<th>Lap</th>

							<th>Duration</th>
							{#if showMetrics}
								{#each metrics as metric}
									<th>{getMetricLabel(metric)}</th>
								{/each}
							{/if}
						</tr>
					</thead>
					<tbody>
						{#each laps as lap, index}
							<tr>
								<td>{index + 1}</td>

								<td>{formatDuration(lap.end - lap.start)}</td>
								{#if showMetrics}
									{#each metrics as metric}
										<td>{calculateLapMetric(lap.start, lap.end, metric) ?? '-'}</td>
									{/each}
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	</details>
{/if}
