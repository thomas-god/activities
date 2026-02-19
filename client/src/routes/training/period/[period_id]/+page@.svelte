<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { getSportCategory, getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import { goto, invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import type { TrainingPeriodDetails } from './+page';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import TrainingMetricsCarousel from '$components/organisms/TrainingMetricsCarousel.svelte';
	import TrainingPeriodStatistics from '$components/organisms/TrainingPeriodStatistics.svelte';
	import ActivityDetails from '$components/pages/ActivityDetails.svelte';
	import {
		fetchActivityDetails,
		type ActivityList,
		type ActivityWithTimeseries
	} from '$lib/api/activities';
	import TrainingMetricsList from '$components/organisms/TrainingMetricsList.svelte';
	import ActivitiesFilters from '$components/molecules/ActivitiesFilters.svelte';
	import CreateTrainingMetric from '$components/organisms/CreateTrainingMetric.svelte';
	import MetricsOrderingDialog from '$components/organisms/MetricsOrderingDialog.svelte';
	import EditPeriodNameModal from '$components/molecules/EditPeriodNameModal.svelte';
	import EditPeriodDatesModal from '$components/molecules/EditPeriodDatesModal.svelte';
	import EditPeriodNoteModal from '$components/molecules/EditPeriodNoteModal.svelte';
	import Timeline from '$components/organisms/Timeline.svelte';

	let { data }: PageProps = $props();

	let showDeleteModal = $state(false);
	let showEditModal = $state(false);
	let showEditNoteModal = $state(false);
	let showEditDatesModal = $state(false);

	let newTrainingMetricDialog: HTMLDialogElement;
	let metricsOrderingDialog: MetricsOrderingDialog;
	let filtersDialog: HTMLDialogElement;

	let chartWidth: number = $state(300);
	let chartHeight = $derived(Math.max(150, Math.min(300, chartWidth * 0.6)));
	let selectedActivityPromise: Promise<ActivityWithTimeseries | null> | null = $state(null);
	let selectedActivityId: string | null = $state(null);
	let screenWidth = $state(0);
	let filteredActivities: ActivityList = $derived([]);

	async function handleDelete(periodId: string) {
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (response.ok) {
			await goto('/training/periods');
		} else {
			throw new Error('Failed to delete training period');
		}
	}

	function openEditModal() {
		showEditModal = true;
	}

	function openEditNoteModal() {
		showEditNoteModal = true;
	}

	function openEditDatesModal() {
		showEditDatesModal = true;
	}

	async function handleUpdateNote(periodId: string, note: string) {
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
			method: 'PATCH',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ note })
		});
		if (response.ok) {
			window.location.reload();
		} else {
			const error = await response.json();
			alert(error.error || 'Failed to update training period note');
			throw new Error(error.error);
		}
	}

	async function handleUpdateDates(periodId: string, start: string, end?: string) {
		const body: { start: string; end?: string } = { start };
		if (end) body.end = end;
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
			method: 'PATCH',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(body)
		});
		if (response.ok) {
			window.location.reload();
		} else {
			const error = await response.json();
			alert(error.error || 'Failed to update training period dates');
			throw new Error(error.error);
		}
	}

	async function handleUpdate(periodId: string, name: string) {
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
			method: 'PATCH',
			credentials: 'include',
			mode: 'cors',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ name })
		});
		if (response.ok) {
			window.location.reload();
		} else {
			const error = await response.json();
			alert(error.error || 'Failed to update training period name');
			throw new Error(error.error);
		}
	}

	const sportsByCategory = (sports: TrainingPeriodDetails['sports']) => {
		// Map category -> { category, icon, sports[], showAll }
		const categorySet: Set<SportCategory | 'Other'> = new Set(sports.categories);
		const map = new Map<
			string,
			{ category: SportCategory | 'Other'; icon: string; sports: string[]; showAll: boolean }
		>();

		// First, seed with explicit categories (these mean "all sports")
		for (const category of sports.categories) {
			map.set(category, {
				category: category,
				icon: getSportCategoryIcon(category),
				sports: [],
				showAll: true
			});
		}

		// Then, process individual sports
		for (const sport of sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				const key = category;
				// If category is already present in categories list, skip individual sport
				if (categorySet.has(key)) {
					continue;
				}
				// Otherwise, add sport to its category group
				if (!map.has(key)) {
					map.set(key, {
						category: key,
						icon: getSportCategoryIcon(category),
						sports: [],
						showAll: false
					});
				}
				map.get(key)!.sports.push(sport);
			} else {
				// Sports without category go to "Other"
				const other = 'Other';
				if (!categorySet.has(other)) {
					if (!map.has(other)) {
						map.set(other, {
							category: other,
							icon: getSportCategoryIcon(null),
							sports: [],
							showAll: false
						});
					}
					map.get(other)!.sports.push(sport);
				}
			}
		}

		return Array.from(map.values());
	};

	const formatPeriodDuration = (start: string, end: string | null): string => {
		const startDate = dayjs(start);
		const endDate = end ? dayjs(end) : dayjs();
		// Add 1 to include the last day (end date is inclusive)
		const days = endDate.diff(startDate, 'day') + 1;

		if (days === 1) return '1 day';
		if (days < 7) return `${days} days`;

		const weeks = Math.floor(days / 7);
		const remainingDays = days % 7;

		if (remainingDays === 0) {
			return weeks === 1 ? '1 week' : `${weeks} weeks`;
		}

		const weeksText = weeks === 1 ? '1 week' : `${weeks} weeks`;
		const daysText = remainingDays === 1 ? '1 day' : `${remainingDays} days`;
		return `${weeksText} ${daysText}`;
	};

	const selectActivityCallback = (activityId: string) => {
		// On mobile, navigate to activity page
		if (screenWidth < 700) {
			goto(`/activity/${activityId}`);
			return;
		}

		// On desktop, load and show activity details in right column
		selectedActivityId = activityId;
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};

	const handleActivityDeleted = (periodId: string) => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		invalidate(`app:training-period:${periodId}`);
	};

	const openMetricsOrderingDialog = () => {
		metricsOrderingDialog.open();
	};
</script>

<svelte:window bind:innerWidth={screenWidth} />

{#await data.periodDetails}
	<div class="flex w-full flex-col items-center p-4 pt-6">
		<div class="loading loading-bars"></div>
	</div>
{:then periodDetails}
	{#if periodDetails !== null}
		<div class="item period-title @container mt-5 rounded-box bg-base-100 p-4 shadow-md">
			<!-- Top row: Icon and Title/Date/Actions -->
			<div class="flex items-center gap-3">
				<!-- Icon -->
				<div class="text-2xl leading-none @lg:text-3xl">🗓️</div>

				<!-- Title and date -->
				<div class="flex-1">
					<div class="text-lg font-semibold @lg:text-xl">{periodDetails.name}</div>
					<div class="flex flex-wrap items-center gap-2 text-xs @lg:text-sm">
						<div class="opacity-70">
							{dayjs(periodDetails.start).format('MMM D, YYYY')} · {periodDetails.end === null
								? 'Ongoing'
								: dayjs(periodDetails.end).format('MMM D, YYYY')}
						</div>
						{#if sportsByCategory(periodDetails.sports).length > 0}
							<div class="flex items-center gap-1.5">
								<span class="opacity-50">·</span>
								{#each sportsByCategory(periodDetails.sports) as group}
									<div
										class="tooltip tooltip-bottom text-base"
										data-tip={group.showAll
											? `${group.category} (all sub-sports)`
											: `${group.category}: ${group.sports.join(', ')}`}
									>
										{group.icon}
									</div>
								{/each}
							</div>
						{:else}
							<div class="opacity-50">· All sports</div>
						{/if}
						<!-- Action menu dropdown (always inline) -->
						<div class="dropdown dropdown-end">
							<div tabindex="0" role="button" class="btn btn-square opacity-100 btn-ghost btn-xs">
								⋮
							</div>
							<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
							<ul
								tabindex="0"
								class="dropdown-content menu z-[1] w-40 rounded-box bg-base-100 p-2 shadow"
							>
								<li>
									<button onclick={openEditModal}>
										<span>✏️</span>
										<span>Edit name</span>
									</button>
								</li>
								<li>
									<button onclick={openEditDatesModal}>
										<span>📅</span>
										<span>Edit dates</span>
									</button>
								</li>
								<li>
									<button onclick={() => (showDeleteModal = true)} class="text-error">
										<span>🗑️</span>
										<span>Delete</span>
									</button>
								</li>
							</ul>
						</div>
					</div>
					<div class="text-xs opacity-70">
						{formatPeriodDuration(periodDetails.start, periodDetails.end)}
					</div>
				</div>
			</div>

			<!-- Period note section -->

			<div class="my-4">
				{#if periodDetails.note}
					<div class="flex items-start gap-2">
						<div class="flex-1 rounded bg-base-200 p-3 text-sm whitespace-pre-wrap">
							{periodDetails.note}
							<button
								class="btn btn-square btn-ghost btn-xs"
								onclick={openEditNoteModal}
								aria-label="Edit note"
							>
								✏️
							</button>
						</div>
					</div>
				{:else}
					<button class="btn gap-2 btn-ghost btn-sm" onclick={openEditNoteModal}>
						<span>📝</span>
						<span>Add period description</span>
					</button>
				{/if}
			</div>

			<div class="rounded bg-base-200 p-4">
				<TrainingPeriodStatistics period={periodDetails} />
			</div>
		</div>

		<div class="period_container">
			{#await data.metrics}
				<div class="flex w-full flex-col items-center p-4 pt-6">
					<div class="loading loading-bars"></div>
				</div>
			{:then metrics}
				<div
					class={`item metrics flex-col rounded-box bg-base-100 shadow-md ${selectedActivityId === null ? 'flex' : 'hidden!'}`}
				>
					{#if metrics.length > 0}
						<div bind:clientWidth={chartWidth}>
							<div class="flex flex-row gap-2 pt-4">
								<h2 class=" pl-4 text-lg font-semibold">Training metrics</h2>
								<div class="dropdown dropdown-end">
									<div
										tabindex="0"
										role="button"
										class="btn btn-square opacity-100 btn-ghost btn-xs"
									>
										⋮
									</div>
									<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
									<ul
										tabindex="0"
										class="dropdown-content menu z-[1] w-40 rounded-box bg-base-100 p-2 shadow"
									>
										<li>
											<button onclick={() => newTrainingMetricDialog.show()}>
												<span>➕</span>
												<span>New metric</span>
											</button>
										</li>
										<li>
											<button onclick={openMetricsOrderingDialog}>
												<span>🔢</span>
												<span>Reorder</span>
											</button>
										</li>
									</ul>
								</div>
							</div>
							{#if screenWidth < 700}
								<TrainingMetricsCarousel
									{metrics}
									height={chartHeight}
									onUpdate={() => invalidate(`app:training-period:${periodDetails.id}`)}
									onDelete={() => invalidate(`app:training-period:${periodDetails.id}`)}
								/>
							{:else}
								<TrainingMetricsList
									{metrics}
									height={chartHeight}
									onUpdate={() => invalidate(`app:training-period:${periodDetails.id}`)}
									onDelete={() => invalidate(`app:training-period:${periodDetails.id}`)}
								/>
							{/if}
						</div>
					{:else}
						<div class="text-center text-sm tracking-wide italic opacity-60">
							No training metrics
						</div>
					{/if}
				</div>

				<MetricsOrderingDialog
					bind:this={metricsOrderingDialog}
					scope={{ type: 'trainingPeriod', trainingPeriodId: periodDetails.id }}
					{metrics}
					onSaved={() => invalidate(`app:training-period:${periodDetails.id}`)}
				/>
			{/await}

			<div
				class={`activity-details rounded-box bg-base-100 pt-4 shadow-md ${selectedActivityId !== null ? 'flex' : 'hidden!'}`}
			>
				{#if selectedActivityPromise}
					{#await selectedActivityPromise}
						<div
							class="flex w-full items-center justify-center rounded-box bg-base-100 p-8 shadow-md"
						>
							<span class="loading loading-lg loading-spinner"></span>
						</div>
					{:then selectedActivity}
						{#if selectedActivity}
							<div class="relative w-full">
								<button onclick={() => (selectedActivityId = null)} class="absolute right-3"
									>X</button
								>
								<ActivityDetails
									activity={selectedActivity}
									onActivityUpdated={() => {
										invalidate(`app:training-period:${periodDetails.id}`);
									}}
									onActivityDeleted={() => handleActivityDeleted(periodDetails.id)}
									compact={true}
								/>
							</div>
						{:else}
							<div
								class="flex items-center justify-center rounded-box bg-base-100 p-8 text-error shadow-md"
							>
								Failed to load activity
							</div>
						{/if}
					{:catch error}
						<div
							class="flex items-center justify-center rounded-box bg-base-100 p-8 text-error shadow-md"
						>
							Failed to load activity: {error.message}
						</div>
					{/await}
				{/if}
			</div>
			{#await data.trainingNotes}
				<div class="flex w-full flex-col items-center p-4 pt-6">
					<div class="loading loading-bars"></div>
				</div>
			{:then notes}
				<!-- Activities section -->
				<div class="item activities rounded-box bg-base-100 p-4 shadow-md">
					<div class="mb-4 flex items-center justify-between">
						<h2 class="text-lg font-semibold">Activities & Notes</h2>
						<button onclick={() => filtersDialog.showModal()} class="btn btn-ghost">⚙️</button>
					</div>

					<Timeline
						activities={filteredActivities}
						{notes}
						{selectedActivityId}
						{selectActivityCallback}
					/>
				</div>
			{/await}
		</div>

		<!-- Activities filters modal -->
		<dialog id="modal-period-filters" class="modal" bind:this={filtersDialog}>
			<div class="modal-box flex flex-col items-center">
				<ActivitiesFilters
					activities={periodDetails.activities}
					onFilterChange={(activities) => {
						filteredActivities = activities;
					}}
					open={true}
				/>
				<button class="btn" onclick={() => filtersDialog.close()}>Close</button>
			</div>
			<!-- To allow closing by clicking outside the modal -->
			<form method="dialog" class="modal-backdrop">
				<button>close</button>
			</form>
		</dialog>

		<EditPeriodNameModal
			bind:isOpen={showEditModal}
			currentName={periodDetails.name}
			onConfirm={(name) => handleUpdate(periodDetails.id, name)}
		/>

		<!-- Delete confirmation modal -->
		<DeleteModal
			bind:isOpen={showDeleteModal}
			title="Delete Training Period"
			description="Are you sure you want to delete this training period ? "
			itemPreview={periodDetails.name}
			onConfirm={() => handleDelete(periodDetails.id)}
		/>

		<EditPeriodDatesModal
			bind:isOpen={showEditDatesModal}
			currentStart={periodDetails.start}
			currentEnd={periodDetails.end}
			onConfirm={(start, end) => handleUpdateDates(periodDetails.id, start, end)}
		/>

		<EditPeriodNoteModal
			bind:isOpen={showEditNoteModal}
			currentNote={periodDetails.note}
			onConfirm={(content) => handleUpdateNote(periodDetails.id, content)}
		/>

		<dialog class="modal" id="create-training-metric-dialog" bind:this={newTrainingMetricDialog}>
			<div class="modal-box max-w-3xl">
				<CreateTrainingMetric
					callback={() => {
						newTrainingMetricDialog.close();
						invalidate(`app:training-period:${periodDetails.id}`);
					}}
					scope={{ kind: 'period', periodId: periodDetails.id }}
				/>
			</div>
			<form method="dialog" class="modal-backdrop">
				<button>close</button>
			</form>
		</dialog>
	{:else}
		<p>placeholder</p>
	{/if}
{/await}

<style>
	.period_container {
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: calc(var(--spacing) * 5);
		margin-top: calc(var(--spacing) * 5);
	}

	.item {
		width: 100%;
	}

	.activity-details {
		display: none;
	}

	@media (min-width: 700px) {
		.period_container {
			display: grid;
			grid-template-columns: repeat(2, minmax(0, 1fr));
			grid-template-rows: 100dvh;
			align-items: start;
			overflow: hidden;
		}

		.metrics {
			display: flex;
			height: 100%;
			overflow-y: auto;
			grid-row: 1;
			grid-column: 2;
			flex-direction: column;
			gap: calc(var(--spacing) * 5);
		}

		.activities {
			grid-row: 1;
			grid-column: 1;
			height: 100%;
			overflow-y: auto;
		}

		.activity-details {
			display: flex;
			height: 100%;
			overflow-y: auto;
			grid-row: 1;
			grid-column: 2;
		}
	}
</style>
