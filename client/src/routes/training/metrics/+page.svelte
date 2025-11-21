<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import DateRangeSelector from '$components/organisms/DateRangeSelector.svelte';
	import { dayjs } from '$lib/duration';
	import type { PageProps } from './$types';
	import TrainingMetricsChartStacked from '$components/organisms/TrainingMetricsChartStacked.svelte';
	import TrainingMetricTitle from '$components/molecules/TrainingMetricTitle.svelte';
	import { setPreference, deletePreference } from '$lib/api';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let showDeleteModal = $state(false);
	let showEditModal = $state(false);
	let isUpdating = $state(false);
	let selectedMetric: { id: string; name: string | null } | null = $state(null);
	let editedName = $state('');

	let dates = $derived({
		start: page.url.searchParams.get('start') as string,
		end: page.url.searchParams.get('end') || dayjs().format('YYYY-MM-DD')
	});

	let favoriteMetricId = $derived(data.preferences.find((p) => p.key === 'favorite_metric')?.value);

	let metricsProps = $derived.by(() => {
		let metrics = [];
		for (let i = 0; i < data.metrics.length; i++) {
			let metric = data.metrics.at(i);
			if (metric === undefined) {
				continue;
			}
			let values = [];
			for (const [group, time_values] of Object.entries(metric.values)) {
				for (const [dt, value] of Object.entries(time_values)) {
					values.push({ time: dt, group, value });
				}
			}

			metrics.push({
				values: values,
				name: metric.name,
				metric: metric.metric,
				granularity: metric.granularity,
				aggregate: metric.aggregate,
				sports: metric.sports,
				groupBy: metric.group_by,
				unit: metric.unit,
				id: metric.id,
				showGroup: metric.group_by !== null
			});
		}
		return metrics;
	});

	const deleteMetricCallback = async (): Promise<void> => {
		if (!selectedMetric) return;

		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${selectedMetric.id}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (res.status === 401) {
			goto('/login');
		}
		showDeleteModal = false;
		selectedMetric = null;
		invalidate('app:training-metrics');
	};

	function openEditModal(metricId: string, metricName: string | null) {
		selectedMetric = { id: metricId, name: metricName };
		editedName = metricName || '';
		showEditModal = true;
	}

	function openDeleteModal(metricId: string, metricName: string | null) {
		selectedMetric = { id: metricId, name: metricName };
		showDeleteModal = true;
	}

	async function handleUpdateName() {
		if (!selectedMetric || !editedName.trim()) {
			alert('Name cannot be empty');
			return;
		}

		isUpdating = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${selectedMetric.id}`, {
				method: 'PATCH',
				credentials: 'include',
				mode: 'cors',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ name: editedName.trim() })
			});

			if (response.ok) {
				showEditModal = false;
				selectedMetric = null;
				invalidate('app:training-metrics');
			} else {
				const error = await response.json();
				alert(error.error || 'Failed to update training metric name');
			}
		} catch (error) {
			alert('Error updating training metric name');
			console.error(error);
		} finally {
			isUpdating = false;
		}
	}

	const toggleFavoriteMetric = async (metricId: string): Promise<void> => {
		if (favoriteMetricId === metricId) {
			// Remove favorite
			await deletePreference(fetch, 'favorite_metric');
		} else {
			// Set as favorite
			await setPreference(fetch, {
				key: 'favorite_metric',
				value: metricId
			});
		}
		invalidate('app:training-metrics');
	};

	$effect(() => {
		// Redirect if no start parameter
		const startDate = page.url.searchParams.get('start');
		if (startDate === null) {
			const now = dayjs();
			const start = encodeURIComponent(now.subtract(1, 'month').format('YYYY-MM-DD'));
			goto(`${page.url.toString()}?start=${start}`);
		}
	});

	const datesUpdateCallback = (newDates: { start: string; end: string }) => {
		let url = page.url.pathname.toString();
		url += `?start=${encodeURIComponent(dayjs(newDates.start).format('YYYY-MM-DD'))}`;
		if (newDates.end !== dayjs().format('YYYY-MM-DD')) {
			// For convenience, don't add end date if it's today
			url += `&end=${encodeURIComponent(dayjs(newDates.end).format('YYYY-MM-DD'))}`;
		}
		goto(url);
	};
</script>

<div class="mx-auto flex flex-col gap-4">
	<DateRangeSelector {dates} {datesUpdateCallback} periods={data.periods} />

	{#each metricsProps as metric}
		<div bind:clientWidth={chartWidth} class="rounded-box bg-base-100 pb-3 shadow-md">
			<div class="relative p-4 text-center">
				<TrainingMetricTitle
					name={metric.name}
					granularity={metric.granularity}
					aggregate={metric.aggregate}
					metric={metric.metric}
					sports={metric.sports}
					groupBy={metric.groupBy}
				/>
				<div class="absolute right-4 bottom-[8px]">
					<!-- Action menu dropdown -->
					<div class="dropdown dropdown-end">
						<div tabindex="0" role="button" class="btn btn-square btn-ghost btn-xs">⋮</div>
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
						<ul
							tabindex="0"
							class="dropdown-content menu z-[1] w-52 rounded-box bg-base-100 p-2 shadow"
						>
							<li>
								<button onclick={() => toggleFavoriteMetric(metric.id)}>
									<span>{favoriteMetricId === metric.id ? '⭐' : '☆'}</span>
									<span
										>{favoriteMetricId === metric.id
											? 'Remove from favorites'
											: 'Set as favorite'}</span
									>
								</button>
							</li>
							<li>
								<button onclick={() => openEditModal(metric.id, metric.name)}>
									<span>✏️</span>
									<span>Edit name</span>
								</button>
							</li>
							<li>
								<button onclick={() => openDeleteModal(metric.id, metric.name)} class="text-error">
									<span>🗑️</span>
									<span>Delete</span>
								</button>
							</li>
						</ul>
					</div>
				</div>
			</div>
			<TrainingMetricsChartStacked
				height={250}
				width={chartWidth}
				values={metric.values}
				unit={metric.unit}
				granularity={metric.granularity}
				format={metric.unit === 's' ? 'duration' : 'number'}
				showGroup={metric.showGroup}
				groupBy={metric.groupBy}
			/>
		</div>
	{/each}
</div>

<!-- Edit name modal -->
{#if showEditModal && selectedMetric}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Metric Name</h3>
			<div class="py-4">
				<label class="input">
					<span class="label">Name</span>
					<input
						type="text"
						bind:value={editedName}
						placeholder="Enter metric name"
						class="w-full"
						disabled={isUpdating}
					/>
				</label>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={() => (showEditModal = false)} disabled={isUpdating}>
					Cancel
				</button>
				<button
					class="btn btn-primary"
					onclick={handleUpdateName}
					disabled={isUpdating || !editedName.trim()}
				>
					{#if isUpdating}
						<span class="loading loading-sm loading-spinner"></span>
						Updating...
					{:else}
						Update
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={() => (showEditModal = false)}>close</button>
		</form>
	</dialog>
{/if}

<!-- Delete confirmation modal -->
{#if selectedMetric}
	<DeleteModal
		bind:isOpen={showDeleteModal}
		title="Delete Training Metric"
		description="Are you sure you want to delete this training metric?"
		itemPreview={selectedMetric.name || 'Unnamed metric'}
		onConfirm={deleteMetricCallback}
	/>
{/if}
