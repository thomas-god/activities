<script lang="ts">
	import TimeseriesChart from '$components/organisms/TimeseriesChart.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditableRpe from '$components/molecules/EditableRpe.svelte';
	import EditableWorkoutType from '$components/molecules/EditableWorkoutType.svelte';
	import EditableNutrition from '$components/molecules/EditableNutrition.svelte';
	import EditableFeedback from '$components/molecules/EditableFeedback.svelte';
	import MetricsMultiSelect from '$components/molecules/MetricsMultiSelect.svelte';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import type { Metric } from '$lib/colors';
	import ActivityStatistics from '$components/organisms/ActivityStatistics.svelte';
	import ActivityLaps from '$components/organisms/ActivityLaps.svelte';
	import ActivityHeader from '$components/organisms/ActivityHeader.svelte';
	import PowerCurve from '$components/organisms/PowerCurve.svelte';
	import ActivityMap from '$components/organisms/ActivityMap.svelte';
	import { convertTimeseriesToActiveTime } from '$lib/timeseries';
	import type { WorkoutType } from '$lib/workout-type';
	import type { Nutrition } from '$lib/nutrition';
	import { sportDisplay } from '$lib/sport';
	import type { ActivityWithTimeseries } from '$lib/api/activities';

	interface Props {
		activity: ActivityWithTimeseries;
		onActivityUpdated: (updatedActivity: ActivityWithTimeseries) => void;
		onActivityDeleted: () => void;
		compact?: boolean;
	}

	let { activity, onActivityDeleted, onActivityUpdated, compact = false }: Props = $props();

	let chartWidth: number = $state(0);
	let showDeleteModal = $state(false);
	let chart: ReturnType<typeof TimeseriesChart> | null = $state(null);

	let selectedLap: ActivityWithTimeseries['timeseries']['laps'][number] | null = $state(null);
	const onLapSelectedCallback = (lap: ActivityWithTimeseries['timeseries']['laps'][number]) => {
		if (chart) {
			chart.zoomToLap(lap);
		}
	};

	// Calculate responsive chart height based on width
	let chartHeight = $derived.by(() => {
		if (chartWidth < 640) {
			// Mobile: smaller height
			return 250;
		} else if (chartWidth < 1024) {
			// Tablet: medium height
			return 350;
		} else {
			// Desktop: full height
			return 400;
		}
	});

	let hasGpsData = $derived(
		'Latitude' in activity.timeseries.metrics &&
			'Longitude' in activity.timeseries.metrics &&
			activity.timeseries.metrics['Latitude'].values.some((v) => v !== null)
	);

	let active_metrics = $derived(convertTimeseriesToActiveTime(activity.timeseries));
	let active_distance = $derived(
		'Distance' in active_metrics.metrics ? active_metrics.metrics['Distance'].values : undefined
	);
	let powerValues = $derived(
		'Power' in active_metrics.metrics ? active_metrics.metrics['Power'].values : null
	);
	let hasPowerData = $derived(powerValues !== null && powerValues.some((v) => v !== null));

	let metricOptions: { option: Metric; display: string }[] = [
		{ option: 'HeartRate', display: 'Heart rate' },
		{ option: 'Speed', display: 'Speed' },
		{ option: 'Power', display: 'Power' },
		{ option: 'Altitude', display: 'Altitude' },
		{ option: 'Cadence', display: 'Cadence' }
	];

	let availableOptions = $derived.by(() => {
		let options = [];
		for (const option of metricOptions) {
			if (
				option.option in active_metrics.metrics &&
				active_metrics.metrics[option.option].values.some((value) => value !== null)
			) {
				options.push(option);
			}
		}
		return options;
	});

	// We use $derived to recalculate when availableOptions changes
	// svelte-ignore state_referenced_locally
	let selectedOptions = $state([availableOptions[0]]);

	let selectedMetrics = $derived.by(() => {
		return selectedOptions.map((option) => {
			return {
				values: active_metrics.metrics[option.option!].values,
				name: option.option,
				unit: active_metrics.metrics[option.option!].unit
			};
		});
	});

	const onDownloadCallback = async (): Promise<void> => {
		const response = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}/download`, {
			method: 'GET',
			mode: 'cors',
			credentials: 'include'
		});
		if (response.status === 401) {
			goto('/login');
			throw new Error('Unauthorized');
		}

		if (!response.ok) {
			throw new Error('Failed to download activities');
		}

		const disposition = response.headers.get('Content-Disposition');
		const filename = disposition?.match(/filename="([^"]+)"/)?.[1] ?? activity.name ?? activity.id;

		const blob = await response.blob();
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		a.click();
		URL.revokeObjectURL(url);
	};

	const deleteActivityCallback = async (): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}`, {
			method: 'DELETE',
			mode: 'cors',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
			return;
		}

		if (res.ok) {
			onActivityDeleted();
		} else {
			throw new Error('Failed to delete activity');
		}
	};

	const openDeleteModal = () => {
		showDeleteModal = true;
	};

	const updateActivityNameCallback = async (newName: string) => {
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${activity.id}?name=${encodeURIComponent(newName)}`,
			{
				method: 'PATCH',
				credentials: 'include'
			}
		);

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			activity.name = newName;
			onActivityUpdated(activity);
		}
	};

	const updateActivityRpeCallback = async (newRpe: number | null) => {
		const rpeParam = newRpe === null ? '0' : newRpe.toString();
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}?rpe=${rpeParam}`, {
			method: 'PATCH',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			activity.rpe = newRpe;
			onActivityUpdated(activity);
		}
	};

	const updateActivityWorkoutTypeCallback = async (newWorkoutType: WorkoutType | null) => {
		const workoutTypeParam = newWorkoutType === null ? '' : newWorkoutType;
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${activity?.id}?workout_type=${encodeURIComponent(workoutTypeParam)}`,
			{
				method: 'PATCH',
				credentials: 'include'
			}
		);

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			activity.workout_type = newWorkoutType;
			onActivityUpdated(activity);
		}
	};

	const updateActivityNutritionCallback = async (newNutrition: Nutrition | null) => {
		const params = new URLSearchParams();

		if (newNutrition === null) {
			params.set('bonk_status', '');
		} else {
			params.set('bonk_status', newNutrition.bonk_status);
			if (newNutrition.details) {
				params.set('nutrition_details', newNutrition.details);
			}
		}

		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}?${params.toString()}`, {
			method: 'PATCH',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			activity.nutrition = newNutrition;
			onActivityUpdated(activity);
		}
	};

	const updateActivityFeedbackCallback = async (newFeedback: string | null) => {
		const body = { feedback: newFeedback };

		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}`, {
			method: 'PATCH',
			credentials: 'include',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(body)
		});

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			activity.feedback = newFeedback;
			onActivityUpdated(activity);
		}
	};

	let sectionClass = $derived(
		compact ? '' : 'rounded-box border border-base-300 bg-base-100 shadow'
	);
</script>

<div class="flex flex-col gap-4">
	<ActivityHeader
		{activity}
		onEditNameCallback={updateActivityNameCallback}
		onDeleteClickedCallback={openDeleteModal}
		{onDownloadCallback}
		{compact}
	/>

	<details class={`collapse-arrow collapse ${sectionClass}`} open>
		<summary class="collapse-title text-lg font-semibold">Session feedbacks</summary>
		<div class="collapse-content flex flex-col gap-3">
			<EditableRpe rpe={activity.rpe} editCallback={updateActivityRpeCallback} />
			<div class="my-0 border-b border-base-300"></div>
			<EditableWorkoutType
				workoutType={activity.workout_type}
				editCallback={updateActivityWorkoutTypeCallback}
			/>
			<div class="my-0 border-b border-base-300"></div>
			<EditableNutrition
				nutrition={activity.nutrition}
				editCallback={updateActivityNutritionCallback}
			/>
			<div class="my-0 border-b border-base-300"></div>
			<EditableFeedback
				feedback={activity.feedback}
				editCallback={updateActivityFeedbackCallback}
			/>
		</div>
	</details>

	<details class={`collapse-arrow collapse ${sectionClass}`} open>
		<summary class="collapse-title text-lg font-semibold">Statistics</summary>
		<div class="collapse-content">
			<ActivityStatistics {activity} />
		</div>
	</details>

	<details class={`collapse-arrow collapse ${sectionClass}`} open>
		<summary class="collapse-title text-lg font-semibold">Metrics</summary>
		<div class="collapse-content px-0">
			<fieldset class="fieldset px-4">
				<MetricsMultiSelect {availableOptions} maxSelected={3} bind:selectedOptions />
			</fieldset>
			{#if selectedMetrics}
				<div class="px-2 pb-2">
					<div class=" w-full overflow-hidden" bind:clientWidth={chartWidth}>
						<TimeseriesChart
							bind:this={chart}
							time={active_metrics.time}
							distance={active_distance}
							metrics={selectedMetrics}
							height={chartHeight}
							width={chartWidth}
							{selectedLap}
						/>
					</div>
				</div>
			{/if}
		</div>
	</details>

	<details class={`collapse-arrow collapse ${sectionClass}`} open>
		<summary class="collapse-title text-lg font-semibold">Laps</summary>
		<div class="collapse-content overflow-x-scroll">
			<ActivityLaps {activity} bind:selectedLap {onLapSelectedCallback} />
		</div>
	</details>

	{#if hasGpsData}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Map</summary>
			<div class="collapse-content px-0">
				<div class="h-80 px-2 pb-2">
					<ActivityMap timeseries={activity.timeseries} />
				</div>
			</div>
		</details>
	{/if}

	{#if hasPowerData}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Power curve</summary>
			<div class="collapse-content px-0">
				<div class="px-2 pb-2">
					<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
						<PowerCurve powerValues={powerValues!} width={chartWidth} height={chartHeight} />
					</div>
				</div>
			</div>
		</details>
	{/if}
</div>

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Activity"
	description="Are you sure you want to delete this activity?"
	itemPreview={activity.name || sportDisplay(activity.sport)}
	onConfirm={deleteActivityCallback}
/>
