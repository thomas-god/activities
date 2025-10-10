<script lang="ts">
	let { callback }: { callback: (payload: Object) => Promise<void> } = $props();

	let formOpen = $state(false);

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

	let statisticSource = $derived.by(() => {
		if (sourceType === 'activity-statistics') {
			return { Statistic: sourceActivityStatistics };
		} else {
			return { Timeseries: [sourceTimeseriesMetric, sourceTimeseriesAggregate] };
		}
	});

	let requestPending = $state(false);

	let metricRequest = $derived.by(() => {
		return {
			source: statisticSource,
			granularity,
			aggregate
		};
	});
</script>

<details class="collapse border border-base-300 bg-base-100" bind:open={formOpen}>
	<summary class="collapse-title font-semibold">Add a new training metric</summary>
	<div class="collapse-content text-sm">
		<fieldset class="fieldset rounded-box bg-base-100 p-2">
			<label class="label" for="source-type">Metric source</label>
			<select class="select" bind:value={sourceType} id="source-type">
				<option value="activity-statistics">Activity statistics</option>
				<option value="timeseries-aggregate">Timeseries aggregate</option>
			</select>

			{#if sourceType === 'activity-statistics'}
				<label class="label" for="source-Activity-statistics">Statistics</label>
				<select
					class="select"
					bind:value={sourceActivityStatistics}
					id="source-Activity-statistics"
				>
					<option value="Calories">Calories</option>
					<option value="Elevation">Elevation gain</option>
					<option value="Distance">Distance</option>
					<option value="Duration">Duration</option>
					<option value="NormalizedPower">Normalized power</option>
				</select>
			{:else}
				<div class="ml-3 flex flex-row gap-3">
					<div>
						<label class="label" for="source-timeseries-metric">Timeseries metric</label>
						<select
							class="select"
							bind:value={sourceTimeseriesMetric}
							id="source-timeseries-metric"
						>
							<option value="Altitude">Altitude</option>
							<option value="Speed">Speed</option>
							<option value="Power">Power</option>
							<option value="HeartRate">Heart rate</option>
							<option value="Cadence">Cadence</option>
						</select>
					</div>

					<div>
						<label class="label" for="source-timeseries-aggregate">Timeseries aggregate</label>
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

			<label class="label" for="metric-granularity">Metric granularity</label>
			<select class="select" bind:value={granularity} id="metric-granularity">
				<option value="Daily">Daily</option>
				<option value="Weekly">Weekly</option>
				<option value="Monthly">Monthly</option>
			</select>

			<label class="label" for="metric-aggregate">Metric aggregate</label>
			<select class="select" bind:value={aggregate} id="metric-aggregate">
				<option value="Max">Maximum value</option>
				<option value="Min">Minimum value</option>
				<option value="Sum">Total</option>
				<option value="Average">Average</option>
			</select>

			<button
				class="btn mt-4 btn-neutral"
				onclick={async () => {
					requestPending = true;
					await callback(metricRequest);
					requestPending = false;
					formOpen = false;
				}}
				disabled={requestPending}
				>Create metric
				{#if requestPending}
					<span class="loading loading-spinner"></span>
				{/if}
			</button>
		</fieldset>
	</div>
</details>
