<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import SportsSelect from '../molecules/SportsSelect.svelte';
	let { callback }: { callback: () => void } = $props();
	let sourceType: 'activity-statistics' | 'timeseries-aggregate' = $state('activity-statistics');
	let sourceActivityStatistics:
		| 'Calories'
		| 'Elevation'
		| 'Distance'
		| 'Duration'
		| 'NormalizedPower' = $state('Calories');
	let sourceTimeseriesMetric: 'Speed' | 'Power' | 'HeartRate' | 'Altitude' | 'Cadence' =
		$state('Power');
	let sourceTimeseriesAggregate: 'Min' | 'Max' | 'Average' | 'Sum' = $state('Average');
	let granularity: 'Daily' | 'Weekly' | 'Monthly' = $state('Weekly');
	let aggregate: 'Min' | 'Max' | 'Average' | 'Sum' = $state('Average');
	let groupBy: 'Sport' | 'SportCategory' | 'WorkoutType' | 'RpeRange' | 'Bonked' = $state('Sport');
	let groupBySelected = $state(false);

	let selectedSports: Sport[] = $state([]);
	let selectedSportCategories: SportCategory[] = $state([]);
	let sportFilterSelected = $state(false);

	let statisticSource = $derived.by(() => {
		if (sourceType === 'activity-statistics') {
			return { Statistic: sourceActivityStatistics };
		} else {
			return { Timeseries: [sourceTimeseriesMetric, sourceTimeseriesAggregate] };
		}
	});

	let requestPending = $state(false);

	let metricRequest = $derived.by(() => {
		let basePayload: {
			source: typeof statisticSource;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters: {};
			group_by?: typeof groupBy;
		} = { source: statisticSource, granularity, aggregate, filters: {} };

		if (sportFilterSelected) {
			const sportFilter = selectedSports.map((sport) => ({
				Sport: sport
			}));
			const sportCategoriesFilter = selectedSportCategories.map((category) => ({
				SportCategory: category
			}));
			const filters = sportFilter.concat(sportCategoriesFilter);
			basePayload = { ...basePayload, filters: { sports: filters } };
		}

		if (groupBySelected) {
			basePayload = { ...basePayload, group_by: groupBy };
		}

		return basePayload;
	});

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	const createMetricCallback = async (payload: Object): Promise<void> => {
		const body = JSON.stringify({ initial_date_range: dates, ...payload });
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric`, {
			body,
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto('/login');
		}
		invalidate('app:training-metrics');
		callback();
	};
</script>

<div class=" text-sm">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<label class="label" for="source-type">Metric source</label>
		<select class="select" bind:value={sourceType} id="source-type">
			<option value="activity-statistics">Activity statistics</option>
			<option value="timeseries-aggregate">Timeseries aggregate</option>
		</select>

		{#if sourceType === 'activity-statistics'}
			<label class="label" for="source-Activity-statistics">Statistics</label>
			<select class="select" bind:value={sourceActivityStatistics} id="source-Activity-statistics">
				<option value="Calories">Calories</option>
				<option value="Elevation">Elevation gain</option>
				<option value="Distance">Distance</option>
				<option value="Duration">Duration</option>
				<option value="NormalizedPower">Normalized power</option>
			</select>
		{:else}
			<div class="ml-3 flex flex-row gap-3">
				<div>
					<label class="label p-1" for="source-timeseries-metric">Timeseries metric</label>
					<select class="select" bind:value={sourceTimeseriesMetric} id="source-timeseries-metric">
						<option value="Altitude">Altitude</option>
						<option value="Speed">Speed</option>
						<option value="Power">Power</option>
						<option value="HeartRate">Heart rate</option>
						<option value="Cadence">Cadence</option>
					</select>
				</div>

				<div>
					<label class="label p-1" for="source-timeseries-aggregate">Timeseries aggregate</label>
					<select
						class="select"
						bind:value={sourceTimeseriesAggregate}
						id="source-timeseries-aggregate"
					>
						<option value="Max">Maximum value</option>
						<option value="Min">Minimum value</option>
						<option value="Sum">Total</option>
						<option value="Average">Average</option>
					</select>
				</div>
			</div>
		{/if}

		<label class="label" for="metric-granularity">Granularity</label>
		<select class="select" bind:value={granularity} id="metric-granularity">
			<option value="Daily">Daily</option>
			<option value="Weekly">Weekly</option>
			<option value="Monthly">Monthly</option>
		</select>

		<label class="label" for="metric-aggregate">Aggregate function</label>
		<select class="select" bind:value={aggregate} id="metric-aggregate">
			<option value="Max">Maximum value</option>
			<option value="Min">Minimum value</option>
			<option value="Sum">Total</option>
			<option value="Average">Average</option>
		</select>

		<div class="mt-2">
			<div class="mb-2 font-semibold">
				Group by
				<input type="checkbox" bind:checked={groupBySelected} class="toggle toggle-sm" />
			</div>
			{#if groupBySelected}
				<select class="select" bind:value={groupBy} id="metric-group-by">
					<option value="Sport">Sport</option>
					<option value="SportCategory">Sport Category</option>
					<option value="WorkoutType">Workout Type</option>
					<option value="RpeRange">RPE Range</option>
					<option value="Bonked">Bonked</option>
				</select>
			{/if}
		</div>

		<details class="collapse-arrow collapse border border-base-300 bg-base-100" open={false}>
			<summary class="collapse-title font-semibold">Filters</summary>
			<div class="collapse-content text-sm">
				<div class="mb-2 font-semibold">
					Sports
					<input type="checkbox" bind:checked={sportFilterSelected} class="toggle toggle-sm" />
				</div>
				{#if sportFilterSelected}
					<SportsSelect bind:selectedSports bind:selectedSportCategories />
				{/if}
			</div>
		</details>

		<button
			class="btn mt-4 btn-neutral"
			onclick={async () => {
				requestPending = true;
				await createMetricCallback(metricRequest);
				requestPending = false;
			}}
			disabled={requestPending}
			>Create metric
			{#if requestPending}
				<span class="loading loading-spinner"></span>
			{/if}
		</button>
	</fieldset>
</div>
