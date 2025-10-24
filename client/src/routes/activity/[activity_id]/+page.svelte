<script lang="ts">
	import { formatDuration, localiseDateTime } from '$lib/duration';
	import TimeseriesChart from '../../../organisms/TimeseriesChart.svelte';
	import type { PageProps } from './$types';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditableString from '../../../molecules/EditableString.svelte';
	import EditableRpe from '../../../molecules/EditableRpe.svelte';
	import EditableWorkoutType from '../../../molecules/EditableWorkoutType.svelte';
	import EditableNutrition from '../../../molecules/EditableNutrition.svelte';
	import MultiSelect from '../../../molecules/MultiSelect.svelte';
	import type { Metric } from '$lib/colors';
	import ActivityStatistics from '../../../organisms/ActivityStatistics.svelte';
	import ActivityLaps, { type LapMetric } from '../../../organisms/ActivityLaps.svelte';
	import { convertTimeseriesToActiveTime } from '$lib/timeseries';
	import { getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import type { WorkoutType } from '$lib/workout-type';
	import type { Nutrition } from '$lib/nutrition';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let summary = $derived.by(() => {
		return {
			sport: data.activity.sport,
			duration: formatDuration(data.activity.duration),
			start_time: data.activity.start_time,
			title:
				data.activity.name === null || data.activity.name === ''
					? data.activity.sport
					: data.activity.name
		};
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
		}

		goto('/');
	};

	const updateActivityNameCallback = async (newName: string) => {
		const res = await fetch(
			`${PUBLIC_APP_URL}/api/activity/${data.activity?.id}?name=${encodeURIComponent(newName)}`,
			{
				method: 'PATCH',
				credentials: 'include'
			}
		);

		if (res.status === 401) {
			goto('/login');
		}
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

	const categoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};

	const getLapMetrics = (category: SportCategory | null): LapMetric[] => {
		switch (category) {
			case 'Running':
				return ['distance', 'speed', 'heartRate'];
			case 'Cycling':
				return ['distance', 'power', 'heartRate'];
			case 'Rowing':
				return ['distance', 'speed', 'power', 'heartRate'];
			case 'Swimming':
				return ['distance', 'speed'];
			case 'Ski':
				return ['distance', 'speed', 'heartRate'];
			case 'Walking':
				return ['distance', 'speed', 'heartRate'];
			case 'Cardio':
				return ['heartRate'];
			case 'Climbing':
				return ['heartRate'];
			case 'TeamSports':
			case 'Racket':
			case 'WaterSports':
				return ['distance', 'speed', 'heartRate'];
			default:
				return ['heartRate'];
		}
	};

	let lapMetrics = $derived(getLapMetrics(data.activity.sport_category));
</script>

<div class="mx-auto mt-1 flex flex-col gap-4 sm:px-4">
	<div
		class={`item mt-5 flex flex-1 items-center bg-base-100 p-3 ${categoryClass(data.activity.sport_category)}`}
	>
		<div class={`icon ${categoryClass(data.activity.sport_category)}`}>
			{getSportCategoryIcon(data.activity.sport_category)}
		</div>
		<div class="flex flex-1 flex-col">
			<div class="mb-1 text-lg font-semibold">
				<EditableString content={summary?.title} editCallback={updateActivityNameCallback} />
			</div>
			<div class="text-xs font-light">
				{localiseDateTime(data.activity.start_time)}
			</div>
		</div>
		<div class="font-semibold sm:text-lg">
			<div>
				{formatDuration(data.activity.duration)}
			</div>
			<!-- <div>45 km</div> -->
		</div>
	</div>

	<div class="rounded-box bg-base-100 p-4 shadow-md">
		<div class="flex flex-col gap-3">
			<EditableRpe rpe={data.activity.rpe} editCallback={updateActivityRpeCallback} />
			<div class="divider my-0"></div>
			<EditableWorkoutType
				workoutType={data.activity.workout_type}
				editCallback={updateActivityWorkoutTypeCallback}
			/>
			<div class="divider my-0"></div>
			<EditableNutrition
				nutrition={data.activity.nutrition}
				editCallback={updateActivityNutritionCallback}
			/>
		</div>
	</div>

	<div>
		<ActivityStatistics activity={data.activity} />
	</div>

	<div class="rounded-box bg-base-100 p-4 pt-0 shadow-md">
		{#if data.activity}
			<fieldset class="fieldset">
				<legend class="fieldset-legend text-lg">Metrics</legend>
				<MultiSelect {availableOptions} maxSelected={3} bind:selectedOptions />
			</fieldset>
			{#if selectedMetrics}
				<div bind:clientWidth={chartWidth}>
					<TimeseriesChart
						time={active_metrics.time}
						metrics={selectedMetrics}
						height={400}
						width={chartWidth}
					/>
				</div>
			{/if}
		{/if}
	</div>

	<div class="rounded-box bg-base-100 p-4 pt-0 shadow-md">
		<ActivityLaps activity={data.activity} metrics={lapMetrics} />
	</div>
</div>

<style>
	.chip-container > :global(div) {
		flex-shrink: 0;
	}

	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.item.cycling {
		border-left-color: var(--color-cycling);
	}

	.item.running {
		border-left-color: var(--color-running);
	}

	.item.other {
		border-left-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
	}

	.icon.cycling {
		background: var(--color-cycling-background);
		color: var(--color-cycling);
	}

	.icon.running {
		background: var(--color-running-background);
		color: var(--color-running);
	}

	.icon.other {
		background: var(--color-other-background);
		color: var(--color-other);
	}
</style>
