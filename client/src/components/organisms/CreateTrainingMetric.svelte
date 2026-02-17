<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { dayjs } from '$lib/duration';
	import { type Sport, type SportCategory } from '$lib/sport';
	import { WORKOUT_TYPE_LABELS, workoutTypeToAPI, type WorkoutType } from '$lib/workout-type';
	import { BONK_STATUS_VALUES, bonkStatusToAPI, type BonkStatus } from '$lib/nutrition';
	import { RPE_VALUES } from '$lib/rpe';
	import SportsSelect from '../molecules/SportsSelect.svelte';
	import TrainingMetricsChartStacked from './TrainingMetricsChartStacked.svelte';

	export type Scope = { kind: 'global' } | { kind: 'period'; periodId: string };

	let { callback, scope = { kind: 'global' } }: { callback: () => void; scope?: Scope } = $props();

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
		// Pace timeseries
		{ id: 'pace-max', source: { Timeseries: ['Pace', 'Max'] } },
		{ id: 'pace-min', source: { Timeseries: ['Pace', 'Min'] } },
		{ id: 'pace-avg', source: { Timeseries: ['Pace', 'Average'] } },
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
	let selectedWorkoutTypes: WorkoutType[] = $state([]);
	let workoutTypeFilterSelected = $state(false);
	let selectedBonkStatus: BonkStatus | null = $state(null);
	let bonkStatusFilterSelected = $state(false);
	let selectedRpes: number[] = $state([]);
	let rpeFilterSelected = $state(false);
	let metricName = $state('');

	let chartWidth: number = $state(0);

	let statisticSource = $derived.by(() => {
		const selectedSource = metricSources.find((s) => s.id === selectedMetricSourceId);
		return selectedSource?.source || { Statistic: 'Duration' };
	});

	// Determine unit and format for preview display
	let previewUnit = $derived.by(() => {
		if (aggregate === 'NumberOfActivities') return 'activities';

		// Map metric sources to units
		const source = metricSources.find((s) => s.id === selectedMetricSourceId);
		if (!source) return 's';

		if ('Statistic' in source.source) {
			const stat = source.source.Statistic;
			if (stat === 'Calories') return 'kcal';
			if (stat === 'Elevation') return 'm';
			if (stat === 'Distance') return 'km';
			if (stat === 'Duration') return 's';
			if (stat === 'NormalizedPower') return 'W';
		} else if ('Timeseries' in source.source) {
			const [metric, _aggregate] = source.source.Timeseries;
			if (metric === 'HeartRate') return 'bpm';
			if (metric === 'Power') return 'W';
			if (metric === 'Speed') return 'km/h';
			if (metric === 'Altitude') return 'm';
			if (metric === 'Cadence') return 'rpm';
			if (metric === 'Distance') return 'km';
			if (metric === 'Pace') return 's/km';
		}

		return 's';
	});

	let previewFormat = $derived.by((): 'number' | 'duration' | 'pace' => {
		if (aggregate === 'NumberOfActivities') return 'number';
		if (previewUnit === 's') return 'duration';
		if (previewUnit === 's/km') return 'pace';
		return 'number';
	});

	let requestPending = $state(false);

	let previewRequest = $derived.by(() => {
		const start = dayjs().subtract(3, 'weeks').format('YYYY-MM-DDTHH:mm:ssZ');
		const end = dayjs().format('YYYY-MM-DDTHH:mm:ssZ');

		let payload: {
			source: typeof statisticSource;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters?: {};
			group_by?: Exclude<typeof groupBy, 'None'>;
			start: string;
			end: string;
		} = { source: statisticSource, granularity, aggregate, start, end };

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
		if (Object.keys(filters).length > 0) {
			payload = { ...payload, filters };
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
			source: typeof statisticSource;
			granularity: typeof granularity;
			aggregate: typeof aggregate;
			filters: {};
			group_by?: Exclude<typeof groupBy, 'None'>;
		} = { name: metricName.trim(), source: statisticSource, granularity, aggregate, filters: {} };

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
			filters.workout_types = selectedWorkoutTypes;
		}
		if (bonkStatusFilterSelected && selectedBonkStatus !== null) {
			filters.bonked = selectedBonkStatus;
		}
		if (rpeFilterSelected && selectedRpes.length > 0) {
			filters.rpes = selectedRpes;
		}
		basePayload = { ...basePayload, filters };

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

<div class="grid grid-cols-1 gap-4 text-sm sm:grid-cols-2">
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

				<div class="mb-2 font-semibold">
					<span class="pr-2"> Filter activities by sports </span>
					<input type="checkbox" bind:checked={sportFilterSelected} class="toggle toggle-sm" />
				</div>
				{#if sportFilterSelected}
					<div class="max-h-96 overflow-scroll">
						<SportsSelect bind:selectedSports bind:selectedSportCategories />
					</div>
				{/if}

				<div class="divider"></div>

				<div class="mb-2 font-semibold">
					<span class="pr-2"> Filter activities by workout type </span>
					<input
						type="checkbox"
						bind:checked={workoutTypeFilterSelected}
						class="toggle toggle-sm"
					/>
				</div>
				{#if workoutTypeFilterSelected}
					<div class="grid grid-cols-2 gap-2">
						{#each WORKOUT_TYPE_LABELS as { value, label } (value)}
							<label class="label cursor-pointer justify-start gap-2">
								<input
									type="checkbox"
									class="checkbox checkbox-sm"
									{value}
									checked={selectedWorkoutTypes.includes(value)}
									onchange={(e) => {
										if (e.currentTarget.checked) {
											selectedWorkoutTypes = [...selectedWorkoutTypes, value];
										} else {
											selectedWorkoutTypes = selectedWorkoutTypes.filter((wt) => wt !== value);
										}
									}}
								/>
								<span class="label-text text-xs">{label}</span>
							</label>
						{/each}
					</div>
				{/if}

				<div class="divider"></div>

				<div class="mb-2 font-semibold">
					<span class="pr-2"> Filter activities by bonk status </span>
					<input type="checkbox" bind:checked={bonkStatusFilterSelected} class="toggle toggle-sm" />
				</div>
				{#if bonkStatusFilterSelected}
					<div class="flex gap-4">
						{#each BONK_STATUS_VALUES as status (status)}
							<label class="label cursor-pointer justify-start gap-2">
								<input
									type="radio"
									name="bonk-status"
									class="radio radio-sm"
									value={status}
									checked={selectedBonkStatus === status}
									onchange={() => {
										selectedBonkStatus = status;
									}}
								/>
								<span class="label-text text-xs capitalize"
									>{status === 'none' ? 'No bonk' : 'Bonked'}</span
								>
							</label>
						{/each}
					</div>
				{/if}

				<div class="divider"></div>

				<div class="mb-2 font-semibold">
					<span class="pr-2"> Filter activities by RPE </span>
					<input type="checkbox" bind:checked={rpeFilterSelected} class="toggle toggle-sm" />
				</div>
				{#if rpeFilterSelected}
					<div class="grid grid-cols-5 gap-2">
						{#each RPE_VALUES as rpe (rpe)}
							<label class="label cursor-pointer justify-start gap-2">
								<input
									type="checkbox"
									class="checkbox checkbox-sm"
									value={rpe}
									checked={selectedRpes.includes(rpe)}
									onchange={(e) => {
										if (e.currentTarget.checked) {
											selectedRpes = [...selectedRpes, rpe];
										} else {
											selectedRpes = selectedRpes.filter((r) => r !== rpe);
										}
									}}
								/>
								<span class="label-text text-xs">{rpe}/10</span>
							</label>
						{/each}
					</div>
				{/if}
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
