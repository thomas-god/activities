<script lang="ts">
	import * as d3 from 'd3';
	import type { TrainingPeriodDetails } from '$lib/api';
	import { dayjs, formatDurationHoursMinutes } from '$lib/duration';
	import { none, some, unwrapOr, type Option } from '$lib/Options';
	import { resolve } from '$app/paths';
	import {
		formatDistance,
		formatElevation,
		formatPeriodDuration,
		periodActivitiesSummary
	} from '$lib/trainingPeriod';

	let { periods }: { periods: TrainingPeriodDetails[] } = $props();

	type TableRow = {
		label: string;
		unit?: string;
		values: Option<string>[];
	};

	let summaries = $derived(periods.map((period) => periodActivitiesSummary(period.activities)));

	let tableRows = $derived.by<TableRow[]>(() => {
		const rows: TableRow[] = [];

		rows.push({
			label: 'Dates',
			values: periods.map((period) =>
				some(
					`${dayjs(period.start).format('MMM D, YYYY')} – ${
						period.end === null ? 'Ongoing' : dayjs(period.end).format('MMM D, YYYY')
					}`
				)
			)
		});

		rows.push({
			label: 'Length',
			values: periods.map((period) => some(formatPeriodDuration(period.start, period.end)))
		});

		rows.push({
			label: 'Activities',
			values: summaries.map((summary) => some(summary.count.toString()))
		});

		rows.push({
			label: 'Activities duration',
			values: summaries.map((summary) => {
				const duration = formatDurationHoursMinutes(summary.duration);
				return duration === '' ? none() : some(duration);
			})
		});

		rows.push({
			label: 'Distance',
			unit: 'km',
			values: summaries.map((summary) => some(formatDistance(summary.distance)))
		});

		rows.push({
			label: 'Elevation',
			unit: 'm',
			values: summaries.map((summary) => some(formatElevation(summary.elevation)))
		});

		return rows;
	});
</script>

<div class="overflow-x-auto">
	<table class="table table-sm">
		<thead>
			<tr>
				<th class="w-32">Statistic</th>
				{#each periods as period, idx (period.id)}
					<th>
						<span
							class="mr-1.5 inline-block h-2.5 w-2.5 rounded-full"
							style="background-color: {d3.schemeTableau10[idx % d3.schemeTableau10.length]}"
						></span>
						<a href={resolve(`/training/period/${period.id}`)} class="link link-hover">
							{period.name}
						</a>
					</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			{#each tableRows as row (row.label)}
				<tr>
					<td class="font-medium">
						{row.label}
						{#if row.unit}
							<span class="ml-0.5 text-xs opacity-60">{row.unit}</span>
						{/if}
					</td>
					{#each row.values as value (value)}
						<td>{unwrapOr(value, '—')}</td>
					{/each}
				</tr>
			{/each}
		</tbody>
	</table>
</div>
