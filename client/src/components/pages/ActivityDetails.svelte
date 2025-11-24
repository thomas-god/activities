<script lang="ts">
	import TimeseriesChart from '$components/organisms/TimeseriesChart.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditableRpe from '$components/molecules/EditableRpe.svelte';
	import EditableWorkoutType from '$components/molecules/EditableWorkoutType.svelte';
	import EditableNutrition from '$components/molecules/EditableNutrition.svelte';
	import EditableFeedback from '$components/molecules/EditableFeedback.svelte';
	import MultiSelect from '$components/molecules/MultiSelect.svelte';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import type { Metric } from '$lib/colors';
	import ActivityStatistics from '$components/organisms/ActivityStatistics.svelte';
	import ActivityLaps from '$components/organisms/ActivityLaps.svelte';
	import ActivityHeader from '$components/organisms/ActivityHeader.svelte';
	import { convertTimeseriesToActiveTime } from '$lib/timeseries';
	import type { WorkoutType } from '$lib/workout-type';
	import type { Nutrition } from '$lib/nutrition';
	import { sportDisplay } from '$lib/sport';
	import type { ActivityDetails } from '$lib/api/activities';

	interface Props {
		activity: ActivityDetails;
		onActivityUpdated: () => void;
		onActivityDeleted: () => void;
		compact?: boolean;
	}

	let { activity, onActivityDeleted, onActivityUpdated, compact = false }: Props = $props();

	let chartWidth: number = $state(0);
	let showDeleteModal = $state(false);

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

	let active_metrics = $derived(convertTimeseriesToActiveTime(activity.timeseries));
	let active_distance = $derived(
		'Distance' in active_metrics.metrics ? active_metrics.metrics['Distance'].values : undefined
	);

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
			onActivityUpdated();
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
			onActivityUpdated();
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
			onActivityUpdated();
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
			onActivityUpdated();
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
			onActivityUpdated();
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
				<MultiSelect {availableOptions} maxSelected={3} bind:selectedOptions />
			</fieldset>
			{#if selectedMetrics}
				<div class="pb-2">
					<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
						<TimeseriesChart
							time={active_metrics.time}
							distance={active_distance}
							metrics={selectedMetrics}
							height={chartHeight}
							width={chartWidth}
						/>
					</div>
				</div>
			{/if}
		</div>
	</details>

	<details class={`collapse-arrow collapse ${sectionClass}`} open>
		<summary class="collapse-title text-lg font-semibold">Laps</summary>
		<div class="collapse-content">
			<ActivityLaps {activity} />
		</div>
	</details>
</div>

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Activity"
	description="Are you sure you want to delete this activity?"
	itemPreview={activity.name || sportDisplay(activity.sport)}
	onConfirm={deleteActivityCallback}
/>
