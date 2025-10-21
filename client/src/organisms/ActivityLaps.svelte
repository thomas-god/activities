<script lang="ts">
	import type { ActivityDetails } from '$lib/api/activities';
	import { dayjs } from '$lib/duration';

	export type LapMetric = 'distance' | 'speed' | 'power' | 'heartRate';

	interface Props {
		activity: ActivityDetails;
		metrics?: LapMetric[];
	}

	let { activity, metrics = [] }: Props = $props();

	const laps = $derived(activity.timeseries.laps);
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
				const lapDistance = (endDistance - startDistance) / 1000; // Convert to km

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
		}
	}
</script>

{#if laps.length > 0}
	<details class="collapse-arrow collapse">
		<summary class="collapse-title text-xl font-medium">Laps</summary>
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
