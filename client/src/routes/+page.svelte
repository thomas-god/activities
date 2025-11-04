<script lang="ts">
	import { goto } from '$app/navigation';
	import PastActivitiesList from '$components/organisms/PastActivitiesList.svelte';
	import type { PageProps } from './$types';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import { dayjs } from '$lib/duration';
	import {
		getSportCategory,
		SportCategories,
		sportCategoryIcons,
		type Sport,
		type SportCategory
	} from '$lib/sport';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);
	let chartHeight = $derived(Math.max(150, Math.min(300, chartWidth * 0.6)));

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);

	// Filter ongoing training periods (no end date or end date >= today)
	let ongoingPeriods = $derived.by(() => {
		const today = dayjs().startOf('day');
		return data.trainingPeriods.filter((period) => {
			if (period.end === null) return true;
			return dayjs(period.end).isAfter(today) || dayjs(period.end).isSame(today);
		});
	});

	let sportIcons = (sports: { sports: Sport[]; categories: SportCategory[] }): string[] => {
		const icons: Set<string> = new Set();

		for (const category of sports.categories) {
			if (SportCategories.includes(category)) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		for (const sport of sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		return Array.from(icons);
	};

	let topMetric = $derived.by(() => {
		let metric = data.metrics.metrics.at(0);
		if (metric === undefined) {
			return undefined;
		}
		let values = [];
		for (const [group, time_values] of Object.entries(metric.values)) {
			for (const [dt, value] of Object.entries(time_values)) {
				values.push({ time: dt, group, value });
			}
		}

		return {
			values: values,
			metric: metric.metric,
			granularity: metric.granularity,
			aggregate: metric.aggregate,
			sports: metric.sports,
			groupBy: metric.group_by,
			unit: metric.unit,
			showGroup: metric.group_by !== null
		};
	});

	const moreActivitiesCallback = () => {
		goto('/history');
	};
</script>

{#if topMetric}
	<div
		bind:clientWidth={chartWidth}
		class="mx-2 mt-5 rounded-box bg-base-100 pb-2 shadow-md sm:mx-auto"
	>
		<div class="mx-3 pt-4 text-center">
			<TrainingMetricTitle
				granularity={topMetric.granularity}
				aggregate={topMetric.aggregate}
				metric={topMetric.metric}
				sports={topMetric.sports}
				groupBy={topMetric.groupBy}
			/>
		</div>
		<TrainingMetricsChartStacked
			height={chartHeight}
			width={chartWidth}
			values={topMetric.values}
			unit={topMetric.unit}
			granularity={topMetric.granularity}
			format={topMetric.unit === 's' ? 'duration' : 'number'}
			showGroup={topMetric.showGroup}
		/>
	</div>
{/if}

{#if ongoingPeriods.length > 0}
	<div class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto">
		<div class="p-4">
			<h2 class="mb-3 text-lg font-semibold">Ongoing Training Periods</h2>
			<div class="flex flex-col gap-2">
				{#each ongoingPeriods as period}
					{@const icons = sportIcons(period.sports)}
					<a
						href={`/training/period/${period.id}`}
						class="flex items-center gap-3 rounded-lg p-3 hover:bg-base-200"
					>
						<div class="text-2xl leading-none">🗓️</div>
						<div class="min-w-0 flex-1">
							<div class="font-semibold">{period.name}</div>
							<div class="text-sm opacity-70">
								{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
									? 'Ongoing'
									: dayjs(period.end).format('MMM D, YYYY')}
							</div>
						</div>
						<div class="flex flex-wrap items-center gap-2">
							{#each icons as icon}
								<div class="text-lg">{icon}</div>
							{:else}
								<div class="text-sm italic opacity-70">All sports</div>
							{/each}
						</div>
					</a>
				{/each}
			</div>
		</div>
	</div>
{/if}

<div class="mx-2 mt-5 sm:mx-auto">
	<PastActivitiesList activityList={sorted_activities} moreCallback={moreActivitiesCallback} />
</div>
