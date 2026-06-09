<script lang="ts">
	import * as d3 from 'd3';
	import { type ActivityWithTimeseries } from '$lib/api';
	import { formatDateTime, formatDuration } from '$lib/duration';
	import { paceToString } from '$lib/speed';

	let { activities }: { activities: ActivityWithTimeseries[] } = $props();

	type TableRow = {
		label: string;
		unit: string;
		legend?: string;
		values: (string | undefined)[];
	};

	let tableRows = $derived.by<TableRow[]>(() => {
		const rows: TableRow[] = [];
		const hasMetric = (key: string) => activities.some((a) => a.metrics[key] !== undefined);

		rows.push({
			label: 'Date',
			unit: '',
			values: activities.map((a) => formatDateTime(a.start_time))
		});

		if (hasMetric('ActiveDuration')) {
			rows.push({
				label: 'Duration',
				unit: '',
				values: activities.map((a) => {
					const v = a.metrics['ActiveDuration'];
					return v !== undefined ? formatDuration(v) : undefined;
				})
			});
		}

		if (hasMetric('Distance')) {
			rows.push({
				label: 'Distance',
				unit: 'km',
				values: activities.map((a) => {
					const v = a.metrics['Distance'];
					return v !== undefined ? (v / 1000).toFixed(3) : undefined;
				})
			});
		}

		if (hasMetric('Calories')) {
			rows.push({
				label: 'Calories',
				unit: 'kcal',
				values: activities.map((a) => {
					const v = a.metrics['Calories'];
					return v !== undefined ? v.toFixed(0) : undefined;
				})
			});
		}

		if (hasMetric('AvgSpeed')) {
			const isRunning = activities.some((a) => a.sport_category === 'Running');
			if (isRunning) {
				rows.push({
					label: 'Pace',
					unit: '/km',
					legend: 'avg',
					values: activities.map((a) => {
						const pace = a.metrics['AvgPace'];
						return pace !== undefined ? paceToString((pace * 1000) / 60) : undefined;
					})
				});
			} else {
				rows.push({
					label: 'Speed',
					unit: 'km/h',
					legend: 'avg',
					values: activities.map((a) => {
						const v = a.metrics['AvgSpeed'];
						return v !== undefined ? (v * 3.6).toFixed(2) : undefined;
					})
				});
			}
		}

		if (hasMetric('Elevation')) {
			rows.push({
				label: 'Elevation',
				unit: 'm',
				values: activities.map((a) => {
					const v = a.metrics['Elevation'];
					return v !== undefined ? v.toFixed(0) : undefined;
				})
			});
		}

		if (hasMetric('AvgHeartRate')) {
			rows.push({
				label: 'Heart rate',
				unit: 'bpm',
				legend: 'avg / max',
				values: activities.map((a) => {
					const avg = a.metrics['AvgHeartRate'];
					const max = a.metrics['MaxHeartRate'];
					if (avg !== undefined && max !== undefined)
						return `${avg.toFixed(0)} / ${max.toFixed(0)}`;
					if (avg !== undefined) return avg.toFixed(0);
					return undefined;
				})
			});
		}

		if (hasMetric('AvgPower')) {
			rows.push({
				label: 'Power',
				unit: 'W',
				legend: 'avg / normalized',
				values: activities.map((a) => {
					const avg = a.metrics['AvgPower'];
					const np = a.metrics['NormalizedPower'];
					if (avg !== undefined && np !== undefined) return `${avg.toFixed(0)} / ${np.toFixed(0)}`;
					if (avg !== undefined) return avg.toFixed(0);
					return undefined;
				})
			});
		}

		return rows;
	});
</script>

{#if tableRows.length > 0}
	<h2 class="pb-1 text-lg">Summary</h2>
	<div class="overflow-x-auto">
		<table class="table table-sm">
			<thead>
				<tr>
					<th class="w-32">Metric</th>
					{#each activities as activity, idx}
						<th>
							<span
								class="mr-1.5 inline-block h-2.5 w-2.5 rounded-full"
								style="background-color: {d3.schemeTableau10[idx % d3.schemeTableau10.length]}"
							></span>
							<a href={`/activity/${activity.id}`} class="link link-hover" target="_blank">
								{activity.name ?? activity.start_time.slice(0, 10)}
							</a>
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each tableRows as row}
					<tr>
						<td class="font-medium">
							{row.label}
							{#if row.unit}
								<span class="ml-0.5 text-xs opacity-60">{row.unit}</span>
							{/if}
							{#if row.legend}
								<div class="text-xs opacity-50">{row.legend}</div>
							{/if}
						</td>
						{#each row.values as value}
							<td>{value ?? '—'}</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
