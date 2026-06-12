<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { getSportCategory, getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import { goto, invalidate } from '$app/navigation';
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
	import MetricsOrderingDialog from '$components/organisms/MetricsOrderingDialog.svelte';
	import EditPeriodNameModal from '$components/molecules/EditPeriodNameModal.svelte';
	import EditPeriodDatesModal from '$components/molecules/EditPeriodDatesModal.svelte';
	import EditPeriodNoteModal from '$components/molecules/EditPeriodNoteModal.svelte';
	import Timeline from '$components/pages/Timeline.svelte';
	import EditButton from '$components/atoms/EditButton.svelte';
	import {
		applyFiltersToSearchParams,
		filtersFromSearchParams,
		type ActivitiesFilters
	} from '$lib/filters';
	import ActivitiesFiltersComponent from '$components/molecules/ActivitiesFilters.svelte';
	import { page } from '$app/state';
	import {
		fetchTrainingMetrics,
		fetchTrainingPeriodDetails,
		fetchTrainingPeriodMetrics,
		fetchTrainingPeriodNotes,
		type MetricsListGrouped,
		type TrainingNotesList,
		type TrainingPeriodDetails
	} from '$lib/api';
	import ImportTrainingMetric from '$components/organisms/ImportTrainingMetric.svelte';
	import { isNone, isSome, none, some, type Option } from '$lib/Options';
	import CreateTrainingMetricFromCollection from '$components/pages/CreateTrainingMetricFromCollection.svelte';

	let period_id = $state(page.params.period_id);

	let pageDetailsPromise: Option<Promise<TrainingPeriodDetails | null>> = $derived(
		period_id === undefined ? none() : some(fetchTrainingPeriodDetails(fetch, period_id))
	);
	const generateMetricsPromise = (): Option<Promise<MetricsListGrouped>> =>
		period_id === undefined ? none() : some(fetchTrainingPeriodMetrics(fetch, period_id));
	const updateMetricsPromise = () => (metricsPromise = generateMetricsPromise());
	let metricsPromise: Option<Promise<MetricsListGrouped>> = $derived(generateMetricsPromise());
	let trainingNotesPromise: Option<Promise<TrainingNotesList>> = $derived(
		period_id === undefined ? none() : some(fetchTrainingPeriodNotes(fetch, period_id))
	);

	let showDeleteModal = $state(false);
	let showEditModal = $state(false);
	let showEditNoteModal = $state(false);
	let showEditDatesModal = $state(false);

	// svelte-ignore non_reactive_update
	let newTrainingMetricDialog: HTMLDialogElement;
	// svelte-ignore non_reactive_update
	let importTrainingMetricDialog: HTMLDialogElement;
	// svelte-ignore non_reactive_update
	let metricsOrderingDialog: MetricsOrderingDialog;

	let chartWidth: number = $state(300);
	let chartHeight = $derived(Math.max(150, Math.min(300, chartWidth * 0.6)));
	let selectedActivityPromise: Promise<ActivityWithTimeseries | null> | null = $state(null);
	let selectedActivityId: string | null = $state(null);
	let screenWidth = $state(0);

	// TODO: get activities directly, not through the period details promise
	let activities: ActivityList = $state([]);
	$effect(() => {
		if (isSome(pageDetailsPromise)) {
			pageDetailsPromise.value.then((details) => (activities = details!.activities));
		}
	});
	let filters = $derived(filtersFromSearchParams(page.url.searchParams));
	let filteredActivities: ActivityList = $state([]);

	const handleFilterChange = (filters: ActivitiesFilters) => {
		const url = new URL(page.url);
		applyFiltersToSearchParams(url.searchParams, filters);
		goto(url, { replaceState: false, keepFocus: true });
	};

	async function handleDelete(periodId: string) {
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${periodId}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (response.ok) {
			await invalidate('app:training-periods');
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
		if (screenWidth < 700) {
			goto(`/activity/${activityId}`);
			return;
		}

		// On desktop, load and show activity details in right column
		selectedActivityId = activityId;
		selectedActivityPromise = fetchActivityDetails(fetch, activityId);
	};

	const handleActivityDeleted = (activiyId: string) => {
		selectedActivityId = null;
		selectedActivityPromise = null;
		activities = activities.filter((activity) => activity.id !== activiyId);
	};

	const handleActivityUpdated = (updatedActivity: ActivityWithTimeseries) => {
		let idx = activities.findIndex((activity) => activity.id === updatedActivity.id);
		if (idx > -1) {
			activities[idx] = updatedActivity;
		}
	};

	const openMetricsOrderingDialog = () => {
		metricsOrderingDialog.open();
	};

	let getGlobalMetricsPromise = $derived.by(async () => {
		if (isNone(pageDetailsPromise)) {
			return Promise.resolve([]);
		}
		const period = await pageDetailsPromise.value;
		if (period === null) {
			return Promise.resolve([]);
		}
		return fetchTrainingMetrics(
			fetch,
			period.start,
			period.end === null ? undefined : period.end,
			'global'
		);
	});
</script>

<svelte:window bind:innerWidth={screenWidth} />

{#if isSome(pageDetailsPromise)}
	{#await pageDetailsPromise.value}
		<div class="flex w-full flex-col items-center p-4 pt-6">
			<div class="loading loading-bars"></div>
		</div>
	{:then periodDetails}
		{#if periodDetails !== null}
			<div class="item period-title @container mt-5 rounded-box bg-base-100 p-4 shadow-md">
				<!-- Top row: Icon and Title/Date/Actions -->
				<div class="flex items-center gap-3">
					<!-- Icon -->
					<div class="text-2xl leading-none @lg:text-3xl">
						<img src="/icons/calendar.svg" class="h-8 w-8 @lg:h-10 @lg:w-10" alt="Calendar icon" />
					</div>

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
											<img src={`/icons/${group.icon}`} class="h-5 w-5" alt="Sport icon" />
										</div>
									{/each}
								</div>
							{:else}
								<div class="opacity-50">· All sports</div>
							{/if}
							<!-- Action menu dropdown (always inline) -->
							<div class="dropdown dropdown-end">
								<button tabindex="0" class="btn px-0.5 btn-xs" aria-label="Options">
									<img src="/icons/menu.svg" class="inline h-7 w-7" alt="Menu icon" />
								</button>
								<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
								<ul
									tabindex="0"
									class="dropdown-content menu z-[1] flex w-40 flex-col items-start rounded-box bg-base-100 p-2 shadow"
								>
									<li class="w-full">
										<button onclick={openEditModal}>
											<img src="/icons/edit.svg" alt="Edit icon" class="h-6 w-6" />Edit name
										</button>
									</li>
									<li class="w-full">
										<button onclick={openEditDatesModal}>
											<img src="/icons/calendar.svg" alt="Calendar icon" class="h-6 w-6" />Edit
											dates
										</button>
									</li>
									<li class="w-full">
										<button onclick={() => (showDeleteModal = true)} class="text-error">
											<img src="/icons/delete.svg" alt="Delete icon" class="h-6 w-6" />Delete
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
							<div class="flex-1 rounded bg-base-200 p-3">
								<div class=" flex flex-row items-center italic">
									<span class="pr-0.5 text-sm"> Period description </span>

									<EditButton callback={openEditNoteModal} />
								</div>
								<div class="text-sm whitespace-pre-wrap">
									{periodDetails.note}
								</div>
							</div>
						</div>
					{:else}
						<button class="btn gap-2 btn-ghost btn-sm" onclick={openEditNoteModal}>
							<img src="/icons/note.svg" class="h-4 w-4" alt="Memo icon" />
							<span>Add period description</span>
						</button>
					{/if}
				</div>

				<div class="rounded bg-base-200 p-4">
					<TrainingPeriodStatistics period={periodDetails} />
				</div>
			</div>

			<div class="period_container">
				{#if isSome(metricsPromise)}
					{#await metricsPromise.value}
						<div class="flex w-full flex-col items-center p-4 pt-6">
							<div class="loading loading-bars"></div>
						</div>
					{:then metrics}
						<div
							class={`item metrics flex-col rounded-box bg-base-100 pb-3 shadow-md ${selectedActivityId === null ? 'flex' : 'hidden!'}`}
						>
							<div bind:clientWidth={chartWidth}>
								<div class="flex flex-row items-center gap-2 pt-4">
									<h2 class=" pl-4 text-lg font-semibold">Training metrics</h2>
									<div class="join">
										<div class="tooltip tooltip-bottom" data-tip="New metric">
											<button
												onclick={() => newTrainingMetricDialog.show()}
												class="btn join-item btn-sm"
											>
												<img src="/icons/plus.svg" class="inline h-5 w-5" alt="Plus sign icon" />
											</button>
										</div>
										<div class="tooltip tooltip-bottom" data-tip="Import metric">
											<button
												onclick={() => importTrainingMetricDialog.show()}
												class="btn join-item btn-sm"
											>
												<img
													src="/icons/import.svg"
													class="inline h-5 w-5"
													alt="Import sign icon"
												/>
											</button>
										</div>
										<div class="tooltip tooltip-bottom" data-tip="Order metrics">
											<button onclick={openMetricsOrderingDialog} class="btn join-item btn-sm">
												<img src="/icons/order.svg" class="inline h-5 w-5" alt="List order icon" />
											</button>
										</div>
									</div>
								</div>
								{#if metrics.length > 0}
									{#if screenWidth < 700}
										<TrainingMetricsCarousel {metrics} height={chartHeight} />
									{:else}
										<TrainingMetricsList
											{metrics}
											height={chartHeight}
											onUpdate={updateMetricsPromise}
											onDelete={updateMetricsPromise}
										/>
									{/if}
								{:else}
									<div class="mt-4 text-center text-sm tracking-wide italic opacity-60">
										No training metrics
									</div>
								{/if}
							</div>
						</div>

						<MetricsOrderingDialog
							bind:this={metricsOrderingDialog}
							scope={{ type: 'trainingPeriod', trainingPeriodId: periodDetails.id }}
							{metrics}
							onSaved={updateMetricsPromise}
						/>
					{/await}
				{/if}

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
									<div class="absolute right-3 join">
										<button
											onclick={() => goto(`/activity/${selectedActivityId}`)}
											class="btn join-item btn-sm"
										>
											<img
												src="/icons/expand.svg"
												alt="Close icon"
												class="inline h-4 w-4"
											/></button
										>
										<button
											onclick={() => (selectedActivityId = null)}
											class="btn join-item btn-sm"
										>
											<img src="/icons/close.svg" alt="Close icon" class="inline h-4 w-4" /></button
										>
									</div>
									<ActivityDetails
										activity={selectedActivity}
										onActivityUpdated={handleActivityUpdated}
										onActivityDeleted={() => handleActivityDeleted(selectedActivity.id)}
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
				{#if isSome(trainingNotesPromise)}
					{#await trainingNotesPromise.value}
						<div class="flex w-full flex-col items-center p-4 pt-6">
							<div class="loading loading-bars"></div>
						</div>
					{:then notes}
						<!-- Activities section -->
						<div class="item activities rounded-box bg-base-100 p-4 shadow-md">
							<div class="mb-4 flex items-center justify-between">
								<h2 class="text-lg font-semibold">Activities & Notes</h2>
								<ActivitiesFiltersComponent
									{activities}
									bind:filteredActivities
									showLabel={false}
									bind:filters={
										() => filters,
										(f) => {
											handleFilterChange(f);
										}
									}
								/>
							</div>

							<Timeline
								activities={filteredActivities}
								{notes}
								{selectedActivityId}
								{selectActivityCallback}
								endDate={periodDetails.end}
							/>
						</div>
					{/await}
				{/if}
			</div>

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
					<CreateTrainingMetricFromCollection
						callback={() => {
							newTrainingMetricDialog.close();
							updateMetricsPromise();
						}}
						scope={{ kind: 'period', periodId: periodDetails.id }}
					/>
				</div>
				<form method="dialog" class="modal-backdrop">
					<button>close</button>
				</form>
			</dialog>

			<dialog
				class="modal"
				id="import-training-metric-dialog"
				bind:this={importTrainingMetricDialog}
			>
				<div class="modal-box max-w-3xl">
					{#await getGlobalMetricsPromise}
						<div class="loading"></div>
					{:then globalMetrics}
						<ImportTrainingMetric
							metrics={globalMetrics}
							period_id={periodDetails.id}
							metricCopiedCallback={updateMetricsPromise}
						/>
					{/await}
				</div>
				<form method="dialog" class="modal-backdrop">
					<button>close</button>
				</form>
			</dialog>
		{:else}
			<p class="pt-4 pl-4 text-sm tracking-wide italic opacity-80">
				Error while loading training period's details. <a class="link" href="/training/periods">
					Go back to periods.
				</a>
			</p>
		{/if}
	{/await}
{/if}

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
			align-items: start;
		}

		.metrics {
			display: flex;
			overflow-y: auto;
			grid-row: 1;
			grid-column: 2;
			flex-direction: column;
			gap: calc(var(--spacing) * 5);
		}

		.activities {
			grid-row: 1;
			grid-column: 1;
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
