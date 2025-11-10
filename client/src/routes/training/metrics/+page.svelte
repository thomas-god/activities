<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import DateRangeSelector from '$components/organisms/DateRangeSelector.svelte';
	import { dayjs } from '$lib/duration';
	import type { PageProps } from './$types';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import { setPreference, deletePreference } from '$lib/api';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	let favoriteMetricId = $derived(data.preferences.find((p) => p.key === 'favorite_metric')?.value);

	let metricsProps = $derived.by(() => {
		let metrics = [];
		for (let i = 0; i < data.metrics.length; i++) {
			let metric = data.metrics.at(i);
			if (metric === undefined) {
				continue;
			}
			let values = [];
			for (const [group, time_values] of Object.entries(metric.values)) {
				for (const [dt, value] of Object.entries(time_values)) {
					values.push({ time: dt, group, value });
				}
			}

			metrics.push({
				values: values,
				metric: metric.metric,
				granularity: metric.granularity,
				aggregate: metric.aggregate,
				sports: metric.sports,
				groupBy: metric.group_by,
				unit: metric.unit,
				id: metric.id,
				showGroup: metric.group_by !== null
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

	const toggleFavoriteMetric = async (metricId: string): Promise<void> => {
		if (favoriteMetricId === metricId) {
			// Remove favorite
			await deletePreference(fetch, 'favorite_metric');
		} else {
			// Set as favorite
			await setPreference(fetch, {
				key: 'favorite_metric',
				value: metricId
			});
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
</script>

<div class="mx-auto flex flex-col gap-4">
	<DateRangeSelector {dates} {datesUpdateCallback} periods={data.periods} />

	{#each metricsProps as metric}
		<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 pb-3 shadow-md">
			<div class="relative p-4 text-center">
				<TrainingMetricTitle
					granularity={metric.granularity}
					aggregate={metric.aggregate}
					metric={metric.metric}
					sports={metric.sports}
					groupBy={metric.groupBy}
				/>
				<div class="absolute right-4 bottom-[8px] flex gap-2">
					<button
						class="btn border-0 bg-base-100 p-0 shadow-none hover:outline-2 hover:outline-base-300"
						onclick={() => toggleFavoriteMetric(metric.id)}
						title={favoriteMetricId === metric.id
							? 'Remove from favorites'
							: 'Set as favorite (shown on homepage)'}
					>
						{favoriteMetricId === metric.id ? '⭐' : '☆'}
					</button>
					<button
						class="btn border-0 bg-base-100 p-0 shadow-none hover:outline-2 hover:outline-base-300"
						onclick={() => deleteMetricCallback(metric.id)}
						title="Delete metric"
					>
						🗑️
					</button>
				</div>
			</div>
			<TrainingMetricsChartStacked
				height={250}
				width={chartWidth}
				values={metric.values}
				unit={metric.unit}
				granularity={metric.granularity}
				format={metric.unit === 's' ? 'duration' : 'number'}
				showGroup={metric.showGroup}
				groupBy={metric.groupBy}
			/>
		</div>
	{/each}
</div>
