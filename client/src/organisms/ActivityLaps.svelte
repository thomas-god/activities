<script lang="ts">
	import type { ActivityDetails } from '$lib/api/activities';
	import dayjs from 'dayjs';
	import duration from 'dayjs/plugin/duration';

	dayjs.extend(duration);

	interface Props {
		activity: ActivityDetails;
	}

	let { activity }: Props = $props();

	const laps = $derived(activity.timeseries.laps);

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
</script>

{#if laps.length > 0}
	<details class="collapse-arrow collapse" open>
		<summary class="collapse-title text-xl font-medium">Laps</summary>
		<div class="collapse-content">
			<div class="overflow-x-auto">
				<table class="table table-sm">
					<thead>
						<tr>
							<th>Lap</th>
							<th>Start</th>
							<th>End</th>
							<th>Duration</th>
						</tr>
					</thead>
					<tbody>
						{#each laps as lap, index}
							<tr>
								<td>{index + 1}</td>
								<td>{formatDuration(lap.start)}</td>
								<td>{formatDuration(lap.end)}</td>
								<td>{formatDuration(lap.end - lap.start)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	</details>
{/if}
