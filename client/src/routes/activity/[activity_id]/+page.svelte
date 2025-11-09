<script lang="ts">
	import TimeseriesChart from '$components/organisms/TimeseriesChart.svelte';
	import type { PageProps } from './$types';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto, invalidate } from '$app/navigation';
	import EditableRpe from '$components/molecules/EditableRpe.svelte';
	import EditableWorkoutType from '$components/molecules/EditableWorkoutType.svelte';
	import EditableNutrition from '$components/molecules/EditableNutrition.svelte';
	import EditableFeedback from '$components/molecules/EditableFeedback.svelte';
	import MultiSelect from '$components/molecules/MultiSelect.svelte';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import type { Metric } from '$lib/colors';
	import ActivityStatistics from '$components/organisms/ActivityStatistics.svelte';
	import ActivityLaps, { type LapMetric } from '$components/organisms/ActivityLaps.svelte';
	import ActivityHeader from '$components/organisms/ActivityHeader.svelte';
	import { convertTimeseriesToActiveTime } from '$lib/timeseries';
	import type { WorkoutType } from '$lib/workout-type';
	import type { Nutrition } from '$lib/nutrition';
	import { sportDisplay } from '$lib/sport';

	let { data }: PageProps = $props();

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

	let active_metrics = $derived(convertTimeseriesToActiveTime(data.activity.timeseries));

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
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}`, {
			method: 'DELETE',
			mode: 'cors',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
			return;
		}

		if (res.ok) {
			await invalidate('app:training-metrics');
			goto('/');
		} else {
			throw new Error('Failed to delete activity');
		}
	};

	const openDeleteModal = () => {
		showDeleteModal = true;
	};

	const updateActivityNameCallback = async (newName: string) => {
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity.id}?name=${encodeURIComponent(newName)}`,
			{
				method: 'PATCH',
				credentials: 'include'
			}
		);

		if (res.status === 401) {
			goto('/login');
		}

		invalidate(`app:activity:${data.activity.id}`);
	};

	const updateActivityRpeCallback = async (newRpe: number | null) => {
		const rpeParam = newRpe === null ? '0' : newRpe.toString();
		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?rpe=${rpeParam}`, {
			method: 'PATCH',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
		}

		// Update local state
		if (res.ok) {
			data.activity.rpe = newRpe;
		}
	};

	const updateActivityWorkoutTypeCallback = async (newWorkoutType: WorkoutType | null) => {
		const workoutTypeParam = newWorkoutType === null ? '' : newWorkoutType;
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?workout_type=${encodeURIComponent(workoutTypeParam)}`,
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
			data.activity.workout_type = newWorkoutType;
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

		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?${params.toString()}`,
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
			data.activity.nutrition = newNutrition;
		}
	};

	const updateActivityFeedbackCallback = async (newFeedback: string | null) => {
		const body = { feedback: newFeedback };

		const res = await fetch(`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}`, {
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
			data.activity.feedback = newFeedback;
		}
	};
</script>

<div class="mx-auto mt-1 flex flex-col gap-4 sm:px-4">
	<ActivityHeader
		activity={data.activity}
		onEditNameCallback={updateActivityNameCallback}
		onDeleteClickedCallback={openDeleteModal}
	/>

	<details
		class="collapse-arrow collapse rounded-box border border-base-300 bg-base-100 shadow"
		open
	>
		<summary class="collapse-title text-lg font-semibold">Session feedbacks</summary>
		<div class="collapse-content">
			<div class="flex flex-col gap-3">
				<EditableRpe rpe={data.activity.rpe} editCallback={updateActivityRpeCallback} />
				<div class="my-0 border-b border-base-300"></div>
				<EditableWorkoutType
					workoutType={data.activity.workout_type}
					editCallback={updateActivityWorkoutTypeCallback}
				/>
				<div class="my-0 border-b border-base-300"></div>
				<EditableNutrition
					nutrition={data.activity.nutrition}
					editCallback={updateActivityNutritionCallback}
				/>
				<div class="my-0 border-b border-base-300"></div>
				<EditableFeedback
					feedback={data.activity.feedback}
					editCallback={updateActivityFeedbackCallback}
				/>
			</div>
		</div>
	</details>

	<ActivityStatistics activity={data.activity} />

	<details
		class="collapse-arrow collapse rounded-box border border-base-300 bg-base-100 shadow"
		open
	>
		<summary class="collapse-title text-lg font-semibold">Metrics</summary>
		<fieldset class="fieldset px-4">
			<MultiSelect {availableOptions} maxSelected={3} bind:selectedOptions />
		</fieldset>
		{#if selectedMetrics}
			<div class="px-4 pb-2">
				<div class="w-full overflow-hidden" bind:clientWidth={chartWidth}>
					<TimeseriesChart
						time={active_metrics.time}
						metrics={selectedMetrics}
						height={chartHeight}
						width={chartWidth}
					/>
				</div>
			</div>
		{/if}
	</details>

	<ActivityLaps activity={data.activity} />
</div>

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Activity"
	description="Are you sure you want to delete this activity?"
	itemPreview={data.activity.name || sportDisplay(data.activity.sport)}
	onConfirm={deleteActivityCallback}
/>
