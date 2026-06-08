<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import { workoutTypeToAPI, type WorkoutType } from '$lib/workout-type';
	import { bonkStatusToAPI, type BonkStatus } from '$lib/nutrition';
	import SportFilter from '../molecules/SportFilter.svelte';
	import WorkoutTypeFilter from '../molecules/WorkoutTypeFilter.svelte';
	import BonkStatusFilter from '../molecules/BonkStatusFilter.svelte';
	import RpeFilter from '../molecules/RpeFilter.svelte';
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';

	export type Scope = { kind: 'global' } | { kind: 'period'; periodId: string };

	let { callback, scope = { kind: 'global' } }: { callback: () => void; scope?: Scope } = $props();

	// Define unified metric sources
	type ActivityMetricOption = {
		id: string;
		metric: string;
	};

	const activityMetricOptions: ActivityMetricOption[] = [
		// Activity statistics
		{ id: 'calories', metric: 'Calories' },
		{ id: 'elevation', metric: 'Elevation' },
		{ id: 'distance', metric: 'Distance' },
		{ id: 'active-duration', metric: 'ActiveDuration' },
		{ id: 'normalized-power', metric: 'NormalizedPower' },
		// Heart rate timeseries
		{ id: 'hr-max', metric: 'MaxHeartRate' },
		{ id: 'hr-min', metric: 'MinHeartRate' },
		{ id: 'hr-avg', metric: 'AvgHeartRate' },
		// Power timeseries
		{ id: 'power-max', metric: 'MaxPower' },
		{ id: 'power-min', metric: 'MinPower' },
		{ id: 'power-avg', metric: 'AvgPower' },
		// Speed timeseries
		{ id: 'speed-max', metric: 'MaxSpeed' },
		{ id: 'speed-min', metric: 'MinSpeed' },
		{ id: 'speed-avg', metric: 'AvgSpeed' },
		// Pace timeseries
		{ id: 'pace-max', metric: 'MaxPace' },
		{ id: 'pace-min', metric: 'MinPace' },
		{ id: 'pace-avg', metric: 'AvgPace' },
		// Altitude timeseries
		{ id: 'altitude-max', metric: 'MaxAltitude' },
		{ id: 'altitude-min', metric: 'MinAltitude' },
		{ id: 'altitude-avg', metric: 'AvgAltitude' },
		// Cadence timeseries
		{ id: 'cadence-max', metric: 'MaxCadence' },
		{ id: 'cadence-min', metric: 'MinCadence' },
		{ id: 'cadence-avg', metric: 'AvgCadence' }
	];

	const granularityValues = ['Daily', 'Weekly', 'Monthly'] as const;
	type Granularity = (typeof granularityValues)[number];

	let selectedMetricSourceId = $state('active-duration');
	let granularity: Granularity = $state('Weekly');
	let aggregate: 'Min' | 'Max' | 'Average' | 'Sum' | 'NumberOfActivities' = $state('Average');
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
	let selectedWorkoutTypes: WorkoutType[] = $state([]);
	let workoutTypeFilterSelected = $state(false);
	let selectedBonkStatus: BonkStatus | null = $state(null);
	let bonkStatusFilterSelected = $state(false);
	let selectedRpes: number[] = $state([]);
	let rpeFilterSelected = $state(false);
	let metricName = $state('');

	let chartWidth: number = $state(0);

	let selectedActivityMetric = $derived.by(() => {
		const selectedMetric = activityMetricOptions.find((s) => s.id === selectedMetricSourceId);
		return selectedMetric?.metric || 'ActiveDuration';
	});

	// Determine unit and format for preview display
	let previewUnit = $derived.by(() => {
		if (aggregate === 'NumberOfActivities') return 'activities';

		// Map metric sources to units
		const source = activityMetricOptions.find((s) => s.id === selectedMetricSourceId);
		if (!source) return 's';

		const metric = source.metric;
		if (metric.includes('Calories')) return 'kcal';
		if (metric.includes('Elevation')) return 'm';
		if (metric.includes('Distance')) return 'km';
		if (metric.includes('Duration')) return 's';
		if (metric.includes('NormalizedPower')) return 'W';
		if (metric.includes('HeartRate')) return 'bpm';
		if (metric.includes('Power')) return 'W';
		if (metric.includes('Speed')) return 'km/h';
		if (metric.includes('Altitude')) return 'm';
		if (metric.includes('Cadence')) return 'rpm';
		if (metric.includes('Distance')) return 'km';
		if (metric.includes('Pace')) return 's/km';

		return 's';
	});

	let previewFormat = $derived.by((): 'number' | 'duration' | 'pace' => {
		if (aggregate === 'NumberOfActivities') return 'number';
		if (previewUnit === 's') return 'duration';
		if (previewUnit === 's/km') return 'pace';
		return 'number';
	});

	let requestPending = $state(false);

	// Build filters object based on selected filter values
	let activeFilters = $derived.by(() => {
		let filters: any = {};

		if (sportFilterSelected) {
			const sportFilter = selectedSports.map((sport) => ({
				Sport: sport
			}));
			const sportCategoriesFilter = selectedSportCategories.map((category) => ({
				SportCategory: category
			}));
			const sportFilters: ({ Sport: Sport } | { SportCategory: SportCategory })[] = [
				...sportFilter,
				...sportCategoriesFilter
			];
			filters.sports = sportFilters;
		}

		if (workoutTypeFilterSelected && selectedWorkoutTypes.length > 0) {
			filters.workout_types = selectedWorkoutTypes.map(workoutTypeToAPI);
		}

		if (bonkStatusFilterSelected && selectedBonkStatus !== null) {
			filters.bonked = bonkStatusToAPI(selectedBonkStatus);
		}

		if (rpeFilterSelected && selectedRpes.length > 0) {
			filters.rpes = selectedRpes;
		}

		return filters;
	});

	let previewRequest = $derived.by(() => {
		const start = dayjs().subtract(3, 'weeks').format('YYYY-MM-DDTHH:mm:ssZ');
		const end = dayjs().format('YYYY-MM-DDTHH:mm:ssZ');

		let payload: {
			metric: string;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters?: {};
			group_by?: Exclude<typeof groupBy, 'None'>;
			start: string;
			end: string;
		} = { metric: selectedActivityMetric, granularity, aggregate, start, end };

		if (Object.keys(activeFilters).length > 0) {
			payload = { ...payload, filters: activeFilters };
		}

		if (groupBy !== 'None') {
			payload = { ...payload, group_by: groupBy };
		}

		return payload;
	});

	// Automatically fetch preview when form values change
	let previewData = $derived.by(() => {
		// Access previewRequest to track its dependencies
		const request = previewRequest;
		return fetchPreview(request);
	});

	// Create a unique key for the preview request to force re-rendering
	let previewKey = $derived(JSON.stringify(previewRequest));

	let metricRequest = $derived.by(() => {
		let basePayload: {
			name: string;
			metric: string;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters: {};
			group_by?: Exclude<typeof groupBy, 'None'>;
		} = {
			name: metricName.trim(),
			metric: selectedActivityMetric,
			granularity,
			aggregate,
			filters: activeFilters
		};

		if (groupBy !== 'None') {
			basePayload = { ...basePayload, group_by: groupBy };
		}

		return basePayload;
	});

	const createMetricCallback = async (payload: Object): Promise<void> => {
		const _scope =
			scope.kind === 'global'
				? { type: 'global' }
				: { type: 'trainingPeriod', trainingPeriodId: scope.periodId };
		const body = JSON.stringify({
			scope: _scope,
			...payload
		});
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

	// TODO: do not run this at component init ?
	const fetchPreview = async (
		request: typeof previewRequest
	): Promise<{ time: string; group: string; value: number }[]> => {
		if (request === null) {
			return [];
		}

		const body = JSON.stringify(request);
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/values`, {
			body,
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto('/login');
			throw new Error('Unauthorized');
		}

		if (res.status !== 200) {
			throw new Error('Failed to fetch preview');
		}

		const data = await res.json();
		// Transform the GroupedMetricValues response to the format expected by the chart
		// Response format: { group_name: { granule: value } }
		const values: { time: string; group: string; value: number }[] = [];
		for (const [group, granuleValues] of Object.entries(data.values)) {
			for (const [time, value] of Object.entries(granuleValues as Record<string, number>)) {
				values.push({ time, group, value });
			}
		}
		return values;
	};
</script>

<div class="grid grid-cols-1 gap-4 text-sm sm:grid-cols-2" id="test-debug">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<legend class="fieldset-legend text-base">New training metric</legend>

		{#if aggregate !== 'NumberOfActivities'}
			<label class="label" for="metric-source">Metric to extract from each activity</label>
			<select class="select" bind:value={selectedMetricSourceId} id="metric-source">
				<optgroup label="Activity Statistics">
					<option value="calories">Calories</option>
					<option value="elevation">Elevation gain</option>
					<option value="distance">Distance</option>
					<option value="active-duration">Duration</option>
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
				<optgroup label="Pace">
					<option value="pace-max">Maximum pace</option>
					<option value="pace-avg">Average pace</option>
					<option value="pace-min">Minimum pace</option>
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

		<label class="label" for="metric-name">Metric name</label>
		<input
			type="text"
			class="input"
			id="metric-name"
			bind:value={metricName}
			placeholder="e.g., Weekly running volume"
			required
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

				<SportFilter
					bind:enabled={sportFilterSelected}
					bind:selectedSports
					bind:selectedSportCategories
				/>

				<div class="divider"></div>

				<WorkoutTypeFilter bind:enabled={workoutTypeFilterSelected} bind:selectedWorkoutTypes />

				<div class="divider"></div>

				<BonkStatusFilter bind:enabled={bonkStatusFilterSelected} bind:selectedBonkStatus />

				<div class="divider"></div>

				<RpeFilter bind:enabled={rpeFilterSelected} bind:selectedRpes />
			</div>
		</details>

		<div class="mt-4">
			<button
				class="btn w-full btn-neutral"
				onclick={async () => {
					requestPending = true;
					await createMetricCallback(metricRequest);
					requestPending = false;
				}}
				disabled={requestPending || !metricName.trim()}
				>Create metric
				{#if requestPending}
					<span class="loading loading-spinner"></span>
				{/if}
			</button>
		</div>
		{#if scope.kind === 'period'}
			<div class="p-1 italic opacity-90">
				This metric will only be visible to the current training period
			</div>
		{/if}
	</fieldset>

	<div class="hidden self-center sm:block">
		{#key previewKey}
			{#await previewData}
				<div class="p-8 text-center">
					<span class="loading loading-lg loading-spinner"></span>
				</div>
			{:then values}
				{#if values.length > 0}
					<div bind:clientWidth={chartWidth}>
						<TrainingMetricsChartStacked
							height={300}
							width={chartWidth}
							{values}
							unit={previewUnit}
							{granularity}
							format={previewFormat}
							showGroup={groupBy !== 'None'}
							groupBy={groupBy !== 'None' ? groupBy : null}
							stacked={aggregate === 'Sum' || aggregate === 'NumberOfActivities'}
						/>
					</div>
				{:else}
					<div class="alert rounded-box alert-info">
						<span>No data available for the selected period and filters.</span>
					</div>
				{/if}
			{:catch error}
				<div class="alert rounded-box alert-error">
					<span>Failed to load preview. Please try again.</span>
				</div>
			{/await}
		{/key}
	</div>
</div>
