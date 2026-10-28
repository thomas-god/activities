<script lang="ts">
	import TimeseriesChart from '#ui/activity/internal/TimeseriesChart.svelte';
	import { PUBLIC_APP_URL } from '$app/env/public';
	import { goto } from '$app/navigation';
	import EditableRpe from '#ui/activity/internal/EditableRpe.svelte';
	import EditableWorkoutType from '#ui/activity/internal/EditableWorkoutType.svelte';
	import EditableNutrition from '#ui/activity/internal/EditableNutrition.svelte';
	import EditableFeedback from '#ui/activity/internal/EditableFeedback.svelte';
	import MetricsMultiSelect from '#ui/activity/internal/MetricsMultiSelect.svelte';
	import DeleteModal from '#ui/shared/DeleteModal.svelte';
	import type { Metric } from '#lib/colors.js';
	import ActivityStatistics from '#ui/activity/internal/ActivityStatistics.svelte';
	import ActivityLaps from '#ui/activity/internal/ActivityLaps.svelte';
	import ActivityHeader from '#ui/activity/internal/ActivityHeader.svelte';
	import DurationCurve from '#ui/activity/DurationCurve.svelte';
	import DurationCurveNotifications from '#ui/activity/DurationCurveNotifications.svelte';
	import ActivityMap from '#ui/activity/internal/ActivityMap.svelte';
	import { feedbackWarnings } from '#ui/activity/internal/feedbackWarnings.js';
	import { TriangleAlert } from '@lucide/svelte';
	import { convertTimeseriesToActiveTime } from '#lib/timeseries.js';
	import type { WorkoutType } from '#lib/workout-type.js';
	import type { Nutrition } from '#lib/nutrition.js';
	import { sportDisplay } from '#lib/sport.js';
	import type { ActivityWithTimeseries, BestDurationCurveReference } from '#lib/api/activities.js';
	import type { DurationCurveReference } from '#lib/durationCurve.js';
	import { asOption, isNone, isSome, none, some, type Option, type Some } from '#lib/Options.js';
	import { resolve } from '$app/paths';

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
	let powerCurve = $derived(
		activity.duration_curves.find((curve) => curve.curve_type === 'Power') ?? null
	);
	let hasPowerData = $derived(powerCurve !== null && powerCurve.values.some((v) => v !== null));
	let paceCurve = $derived(
		activity.duration_curves.find((curve) => curve.curve_type === 'Pace') ?? null
	);
	let hasPaceData = $derived(paceCurve !== null && paceCurve.values.some((v) => v !== null));

	// The training context always carries the 12-week reference and, when the activity
	// was opened from a training period, the period reference as well.
	let bestCurveReferences = $derived(activity.training_context.best_duration_curves);

	type ReferenceKind = BestDurationCurveReference['kind'];

	/** Generic label used in the reference toggle and notification titles. */
	const referenceKindLabel = (reference: BestDurationCurveReference): string =>
		reference.kind === 'twelve_week' ? '12-week' : 'Period';

	/** Richer label used in the chart legend, e.g. the training period name. */
	const referenceLegendLabel = (reference: BestDurationCurveReference): string =>
		reference.kind === 'twelve_week' ? '12-week' : (reference.label ?? 'period');

	// Default to the period reference when the activity was opened from a period.
	// svelte-ignore state_referenced_locally
	let selectedReferenceKind = $state<ReferenceKind>(
		bestCurveReferences.some((reference) => reference.kind === 'training_period')
			? 'training_period'
			: 'twelve_week'
	);

	let selectedReference = $derived(
		bestCurveReferences.find((reference) => reference.kind === selectedReferenceKind) ??
			bestCurveReferences[0] ??
			null
	);

	let hasMultipleReferences = $derived(bestCurveReferences.length > 1);

	// The training context carries one best curve per curve type (e.g. power and pace):
	// reconcile by keeping the one matching the type of the displayed curve.
	let bestPowerCurve = $derived(
		selectedReference?.curves.find((curve) => curve.curve_type === 'Power') ?? null
	);
	let hasBestPowerCurve = $derived(
		bestPowerCurve !== null && bestPowerCurve.values.some((v) => v !== null)
	);
	let bestPaceCurve = $derived(
		selectedReference?.curves.find((curve) => curve.curve_type === 'Pace') ?? null
	);
	let hasBestPaceCurve = $derived(
		bestPaceCurve !== null && bestPaceCurve.values.some((v) => v !== null)
	);

	// Notification groups, one per reference that has a non-empty curve for the
	// displayed type. All groups are shown regardless of the selected reference.
	const referenceGroups = (kind: 'Power' | 'Pace'): DurationCurveReference[] =>
		bestCurveReferences.flatMap((reference) => {
			const curve = reference.curves.find((curve) => curve.curve_type === kind);
			return curve && curve.values.some((value) => value !== null)
				? [{ label: referenceKindLabel(reference), values: curve.values }]
				: [];
		});

	let powerCurveReferences = $derived(referenceGroups('Power'));
	let paceCurveReferences = $derived(referenceGroups('Pace'));
	let activityDuration = $derived(activity.metrics['Duration']?.value ?? null);
	let averagePower = $derived(activity.metrics['AvgPower']?.value ?? null);
	let averagePace = $derived(activity.metrics['AvgSpeed']?.value ?? null);
	let activityWeight = $derived(asOption(activity.training_context.weight));

	let feedbackWarningLabels = $derived(feedbackWarnings(activity.training_context.hooper_index));

	interface MetricOption {
		option: Metric;
		display: string;
	}
	let metricOptions: MetricOption[] = [
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
	let selectedOptions: Option<MetricOption[]> = $state(
		availableOptions.length > 0 ? some([availableOptions[0]]) : none()
	);

	let selectedMetrics = $derived.by(() => {
		if (isNone(selectedOptions)) {
			return;
		}
		return selectedOptions.value.map((option) => {
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
			goto(resolve('login'));
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
			goto(resolve('login'));
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
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity.id}`, {
			method: 'PATCH',
			credentials: 'include',
			body: JSON.stringify({ name: newName }),
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto(resolve('login'));
		}

		// Update local state
		if (res.ok) {
			activity.name = newName;
			onActivityUpdated(activity);
		}
	};

	const updateActivityRpeCallback = async (newRpe: number | null) => {
		const rpeParam = newRpe === null ? 0 : newRpe;
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}`, {
			method: 'PATCH',
			credentials: 'include',
			body: JSON.stringify({ rpe: rpeParam }),
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto(resolve('login'));
		}

		// Update local state
		if (res.ok) {
			activity.rpe = newRpe;
			onActivityUpdated(activity);
		}
	};

	const updateActivityWorkoutTypeCallback = async (newWorkoutType: WorkoutType | null) => {
		const workoutTypeParam = newWorkoutType === null ? '' : newWorkoutType;
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}`, {
			method: 'PATCH',
			credentials: 'include',
			body: JSON.stringify({ workout_type: workoutTypeParam }),
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto(resolve('login'));
		}

		// Update local state
		if (res.ok) {
			activity.workout_type = newWorkoutType;
			onActivityUpdated(activity);
		}
	};

	const updateActivityNutritionCallback = async (newNutrition: Nutrition | null) => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${activity?.id}`, {
			method: 'PATCH',
			credentials: 'include',
			body: JSON.stringify({ nutrition: newNutrition }),
			headers: { 'Content-Type': 'application/json' }
		});

		if (res.status === 401) {
			goto(resolve('login'));
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
			goto(resolve('login'));
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

{#snippet referenceToggle()}
	{#if hasMultipleReferences}
		<div class="flex items-center gap-2 px-2 pb-1 text-xs">
			<span class="opacity-70">Reference</span>
			<div class="join">
				{#each bestCurveReferences as reference (reference.kind)}
					<button
						class="btn join-item btn-xs"
						class:btn-primary={selectedReferenceKind === reference.kind}
						class:btn-outline={selectedReferenceKind !== reference.kind}
						onclick={() => (selectedReferenceKind = reference.kind)}
					>
						{referenceKindLabel(reference)}
					</button>
				{/each}
			</div>
		</div>
	{/if}
{/snippet}

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
			{#if feedbackWarningLabels.length > 0}
				<div class="flex flex-wrap gap-2">
					{#each feedbackWarningLabels as warning (warning.kind)}
						<span
							class="badge gap-1 badge-sm"
							class:badge-warning={warning.severity === 'warning'}
							class:badge-error={warning.severity === 'alert'}
							data-testid={`feedback-warning-${warning.kind}`}
						>
							<TriangleAlert class="h-3.5 w-3.5" />
							{warning.label}
						</span>
					{/each}
				</div>
			{/if}
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
		<div class="collapse-content"><ActivityStatistics {activity} /></div>
	</details>

	{#if isSome(selectedOptions)}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Metrics</summary>
			<div class="collapse-content px-0">
				<fieldset class="fieldset px-4">
					<MetricsMultiSelect
						{availableOptions}
						maxSelected={3}
						bind:selectedOptions={
							() => (selectedOptions as Some<MetricOption[]>).value,
							(v) => (selectedOptions = some(v))
						}
					/>
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
	{/if}

	{#if activity.timeseries.laps.length > 0}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Laps</summary>
			<div class="collapse-content overflow-x-scroll">
				<ActivityLaps {activity} bind:selectedLap {onLapSelectedCallback} />
			</div>
		</details>
	{/if}

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

	{#if hasPaceData}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Pace curve</summary>
			<div class="collapse-content px-0">
				{@render referenceToggle()}
				<div class="px-2 pb-2">
					<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
						<DurationCurve
							kind="pace"
							curveValues={paceCurve!.values}
							bestCurveValues={hasBestPaceCurve ? bestPaceCurve!.values : undefined}
							bestCurveLabel={selectedReference
								? referenceLegendLabel(selectedReference)
								: undefined}
							activityDuration={activityDuration ?? undefined}
							averageValue={averagePace * 3.6}
							width={chartWidth}
							height={chartHeight}
							weight={activityWeight}
						/>
					</div>
				</div>
				<DurationCurveNotifications
					curveValues={paceCurve!.values}
					references={paceCurveReferences}
					unit={paceCurve!.unit}
				/>
			</div>
		</details>
	{/if}

	{#if hasPowerData}
		<details class={`collapse-arrow collapse ${sectionClass}`} open>
			<summary class="collapse-title text-lg font-semibold">Power curve</summary>
			<div class="collapse-content px-0">
				{@render referenceToggle()}
				<div class="px-2 pb-2">
					<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
						<DurationCurve
							kind="power"
							curveValues={powerCurve!.values}
							bestCurveValues={hasBestPowerCurve ? bestPowerCurve!.values : undefined}
							bestCurveLabel={selectedReference
								? referenceLegendLabel(selectedReference)
								: undefined}
							activityDuration={activityDuration ?? undefined}
							averageValue={averagePower}
							width={chartWidth}
							height={chartHeight}
							weight={activityWeight}
						/>
					</div>
				</div>
				<DurationCurveNotifications
					curveValues={powerCurve!.values}
					references={powerCurveReferences}
					unit={powerCurve!.unit}
				/>
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
