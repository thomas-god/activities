<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import DateRange from '../../../molecules/DateRange.svelte';
	import TrainingMetricsChart from '../../../organisms/TrainingMetricsChart.svelte';
	import type { PageProps } from '../$types';
	import { dayjs } from '$lib/duration';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	let metricsProps = $derived.by(() => {
		let metrics = [];
		for (let i = 0; i < data.metrics.length; i++) {
			let metric = data.metrics.at(i);
			if (metric === undefined) {
				continue;
			}
			let values = [];
			for (const dt in metric.values) {
				values.push({ time: dt, value: metric.values[dt] });
			}

			metrics.push({
				values: values,
				title: `${metric.metric} (${metric.granularity}) ${metric.sports.length > 0 ? '[' + metric.sports.join(', ') + ']' : ''}`,
				unit: metric.unit,
				id: metric.id,
				granularity: metric.granularity
			});
		}
		return metrics;
	});

	const deleteMetricCallback = async (metric: string): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (res.status === 401) {
			goto('/login');
		}
		invalidate('app:training-metrics');
	};

	$effect(() => {
		// Redirect if no start parameter
		const startDate = page.url.searchParams.get('start');
		if (startDate === null) {
			const now = dayjs();
			const start = encodeURIComponent(now.subtract(1, 'month').format('YYYY-MM-DD'));
			goto(`${page.url.toString()}?start=${start}`);
		}
	});

	const datesUpdateCallback = (newDates: { start: string; end: string }) => {
		let url = page.url.pathname.toString();
		url += `?start=${encodeURIComponent(dayjs(newDates.start).format('YYYY-MM-DD'))}`;
		if (newDates.end !== dayjs().format('YYYY-MM-DD')) {
			// For convenience, don't add end date if it's today
			url += `&end=${encodeURIComponent(dayjs(newDates.end).format('YYYY-MM-DD'))}`;
		}
		goto(url);
	};

	const pastXWeeks = (numberOfWeek: number) => {
		const now = dayjs().startOf('day');
		const start = now.subtract(numberOfWeek, 'week');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};

	const thisYear = () => {
		const now = dayjs().startOf('day');
		const start = now.startOf('year');
		datesUpdateCallback({ start: start.toISOString(), end: now.toISOString() });
	};
</script>

<div class="mx-auto mt-4 flex flex-col gap-4 sm:mt-8">
	<div class="rounded-box bg-base-100 shadow-md">
		<DateRange bind:dates={() => dates, datesUpdateCallback} />
		<div class="flex flex-row flex-wrap items-center gap-2 p-2">
			<button class="btn btn-sm sm:btn-md" onclick={() => pastXWeeks(4)}>Last 4 weeks</button>
			<button class="btn btn-sm sm:btn-md" onclick={() => pastXWeeks(12)}>Last 12 weeks</button>
			<button class="btn btn-sm sm:btn-md" onclick={thisYear}>This year</button>
			<select class="select select-sm">
				<option disabled selected>Training period</option>
				{#each data.periods as period}
					<option
						onclick={() =>
							datesUpdateCallback({
								start: period.start,
								end: period.end === null ? dayjs().toISOString() : period.end
							})}>{period.name}</option
					>
				{:else}
					<option disabled class="italic">No training periods</option>
				{/each}
			</select>
		</div>
	</div>

	{#each metricsProps as metric}
		<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 shadow-md">
			<div class="relative p-4 text-center">
				<div>
					{metric.title}
				</div>
				<button
					class="btn absolute right-4 bottom-[8px] border-0 bg-base-100 p-0 shadow-none hover:outline-2 hover:outline-base-300"
					onclick={() => deleteMetricCallback(metric.id)}>🗑️</button
				>
			</div>
			<TrainingMetricsChart
				height={250}
				width={chartWidth}
				values={metric.values}
				unit={metric.unit}
				granularity={metric.granularity}
				format={metric.unit === 's' ? 'duration' : 'number'}
			/>
		</div>
	{/each}
</div>
