<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import SportsSelect from '../molecules/SportsSelect.svelte';
	let { callback }: { callback: () => void } = $props();

	// Define unified metric sources
	type MetricSourceOption = {
		id: string;
		source: { Statistic: string } | { Timeseries: [string, string] };
	};

	const metricSources: MetricSourceOption[] = [
		// Activity statistics
		{ id: 'calories', source: { Statistic: 'Calories' } },
		{ id: 'elevation', source: { Statistic: 'Elevation' } },
		{ id: 'distance', source: { Statistic: 'Distance' } },
		{ id: 'duration', source: { Statistic: 'Duration' } },
		{ id: 'normalized-power', source: { Statistic: 'NormalizedPower' } },
		// Heart rate timeseries
		{ id: 'hr-max', source: { Timeseries: ['HeartRate', 'Max'] } },
		{ id: 'hr-min', source: { Timeseries: ['HeartRate', 'Min'] } },
		{ id: 'hr-avg', source: { Timeseries: ['HeartRate', 'Average'] } },
		// Power timeseries
		{ id: 'power-max', source: { Timeseries: ['Power', 'Max'] } },
		{ id: 'power-min', source: { Timeseries: ['Power', 'Min'] } },
		{ id: 'power-avg', source: { Timeseries: ['Power', 'Average'] } },
		// Speed timeseries
		{ id: 'speed-max', source: { Timeseries: ['Speed', 'Max'] } },
		{ id: 'speed-min', source: { Timeseries: ['Speed', 'Min'] } },
		{ id: 'speed-avg', source: { Timeseries: ['Speed', 'Average'] } },
		// Altitude timeseries
		{ id: 'altitude-max', source: { Timeseries: ['Altitude', 'Max'] } },
		{ id: 'altitude-min', source: { Timeseries: ['Altitude', 'Min'] } },
		{ id: 'altitude-avg', source: { Timeseries: ['Altitude', 'Average'] } },
		// Cadence timeseries
		{ id: 'cadence-max', source: { Timeseries: ['Cadence', 'Max'] } },
		{ id: 'cadence-min', source: { Timeseries: ['Cadence', 'Min'] } },
		{ id: 'cadence-avg', source: { Timeseries: ['Cadence', 'Average'] } }
	];

	const granularityValues = ['Daily', 'Weekly', 'Monthly'] as const;
	type Granularity = (typeof granularityValues)[number];

	let selectedMetricSourceId = $state('duration');
	let granularity: Granularity = $state('Weekly');
	let aggregate: 'Min' | 'Max' | 'Average' | 'Sum' | 'NumberOfActivities' = $state('Sum');
	let groupBy: 'None' | 'Sport' | 'SportCategory' | 'WorkoutType' | 'RpeRange' | 'Bonked' =
		$state('None');

	const granularityDisplay: Record<Granularity, string> = {
		Daily: 'day',
		Monthly: 'month',
		Weekly: 'week'
	};

	let selectedSports: Sport[] = $state([]);
	let selectedSportCategories: SportCategory[] = $state([]);
	let sportFilterSelected = $state(false);
	let metricName = $state('');

	let statisticSource = $derived.by(() => {
		const selectedSource = metricSources.find((s) => s.id === selectedMetricSourceId);
		return selectedSource?.source || { Statistic: 'Duration' };
	});

	let requestPending = $state(false);

	let metricRequest = $derived.by(() => {
		let basePayload: {
			name?: string;
			source: typeof statisticSource;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters: {};
			group_by?: Exclude<typeof groupBy, 'None'>;
		} = { source: statisticSource, granularity, aggregate, filters: {} };

		if (metricName.trim()) {
			basePayload = { ...basePayload, name: metricName.trim() };
		}

		if (sportFilterSelected) {
			const sportFilter = selectedSports.map((sport) => ({
				Sport: sport
			}));
			const sportCategoriesFilter = selectedSportCategories.map((category) => ({
				SportCategory: category
			}));
			const filters: ({ Sport: Sport } | { SportCategory: SportCategory })[] = [
				...sportFilter,
				...sportCategoriesFilter
			];
			basePayload = { ...basePayload, filters: { sports: filters } };
		}

		if (groupBy !== 'None') {
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
		<legend class="fieldset-legend text-base">New training metric</legend>

		{#if aggregate !== 'NumberOfActivities'}
			<label class="label" for="metric-source">Metric to extract from each activity</label>
			<select class="select" bind:value={selectedMetricSourceId} id="metric-source">
				<optgroup label="Activity Statistics">
					<option value="calories">Calories</option>
					<option value="elevation">Elevation gain</option>
					<option value="distance">Distance</option>
					<option value="duration">Duration</option>
					<option value="normalized-power">Normalized power</option>
				</optgroup>
				<optgroup label="Heart Rate">
					<option value="hr-max">Maximum heart rate</option>
					<option value="hr-avg">Average heart rate</option>
					<option value="hr-min">Minimum heart rate</option>
				</optgroup>
				<optgroup label="Power">
					<option value="power-max">Maximum power</option>
					<option value="power-avg">Average power</option>
					<option value="power-min">Minimum power</option>
				</optgroup>
				<optgroup label="Speed">
					<option value="speed-max">Maximum speed</option>
					<option value="speed-avg">Average speed</option>
					<option value="speed-min">Minimum speed</option>
				</optgroup>
				<optgroup label="Altitude">
					<option value="altitude-max">Maximum altitude</option>
					<option value="altitude-avg">Average altitude</option>
					<option value="altitude-min">Minimum altitude</option>
				</optgroup>
				<optgroup label="Cadence">
					<option value="cadence-max">Maximum cadence</option>
					<option value="cadence-avg">Average cadence</option>
					<option value="cadence-min">Minimum cadence</option>
				</optgroup>
			</select>
		{/if}

		<label class="label" for="metric-granularity">Group activities by</label>
		<select class="select" bind:value={granularity} id="metric-granularity">
			<option value="Daily">Day</option>
			<option value="Weekly">Week</option>
			<option value="Monthly">Month</option>
		</select>

		<label class="label" for="metric-aggregate"
			>How to aggregate each metric from the {granularityDisplay[granularity]}</label
		>
		<select class="select" bind:value={aggregate} id="metric-aggregate">
			<option value="Max">Maximum value</option>
			<option value="Min">Minimum value</option>
			<option value="Sum">Total</option>
			<option value="Average">Average</option>
			<option value="NumberOfActivities">Number of activities</option>
		</select>

		<label class="label" for="metric-name">Metric name (optional)</label>
		<input
			type="text"
			class="input"
			id="metric-name"
			bind:value={metricName}
			placeholder="e.g., Weekly running volume"
		/>

		<details class="collapse-arrow collapse mt-3 border border-base-300 bg-base-100" open={false}>
			<summary class="collapse-title font-semibold">Groups and filters</summary>
			<div class="collapse-content text-sm">
				<div class="mb-2">
					<label class="label mb-1.5 text-xs" for="metric-group-by"
						>Additionally group activities by</label
					>
					<select class="select w-full" bind:value={groupBy} id="metric-group-by">
						<option value="None">No grouping</option>
						<option value="Sport">Sport</option>
						<option value="SportCategory">Sport Category</option>
						<option value="WorkoutType">Workout Type</option>
						<option value="RpeRange">RPE Range</option>
						<option value="Bonked">Bonked</option>
					</select>
				</div>

				<div class="divider"></div>

				<div class="mb-2 font-semibold">
					<span class="pr-2"> Filter activities by sports </span>
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
