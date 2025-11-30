<script lang="ts">
	import {
		getMetricsOrdering,
		setMetricsOrdering,
		type MetricsOrderingScope
	} from '$lib/api/training-metrics-ordering';
	import { aggregateFunctionDisplay, type MetricAggregateFunction } from '$lib/metric';

	interface Props {
		scope: MetricsOrderingScope;
		metrics: Array<{
			id: string;
			name: string | null;
			granularity: string;
			aggregate: MetricAggregateFunction;
			metric: string;
		}>;
		onSaved?: () => void;
	}

	let { scope, metrics, onSaved }: Props = $props();

	let dialog: HTMLDialogElement;
	let metricsOrder = $state<string[]>([]);
	let isLoadingOrder = $state(false);
	let isSavingOrder = $state(false);
	let draggedIndex = $state<number | null>(null);

	const fetchMetricsOrdering = async () => {
		isLoadingOrder = true;
		try {
			const serverOrder = await getMetricsOrdering(scope);

			if (serverOrder !== null) {
				// Get all current metric IDs
				const allMetricIds = metrics.map((m) => m.id);

				// Find metrics not in the server ordering
				const metricsNotInOrder = allMetricIds.filter((id) => !serverOrder.includes(id));

				// Merge: server order first, then metrics not in ordering
				metricsOrder = [...serverOrder, ...metricsNotInOrder];
			} else {
				// If fetch fails, use all metrics in current order
				metricsOrder = metrics.map((m) => m.id);
			}
		} finally {
			isLoadingOrder = false;
		}
	};

	const saveMetricsOrdering = async () => {
		isSavingOrder = true;
		try {
			const success = await setMetricsOrdering(scope, metricsOrder);
			if (success) {
				dialog.close();
				onSaved?.();
			}
		} finally {
			isSavingOrder = false;
		}
	};

	export const open = async () => {
		await fetchMetricsOrdering();
		dialog.showModal();
	};

	export const close = () => {
		dialog.close();
	};

	const handleDragStart = (index: number) => {
		draggedIndex = index;
	};

	const handleDragOver = (e: DragEvent, index: number) => {
		e.preventDefault();
		if (draggedIndex === null || draggedIndex === index) return;

		const newOrder = [...metricsOrder];
		const [draggedItem] = newOrder.splice(draggedIndex, 1);
		newOrder.splice(index, 0, draggedItem);
		metricsOrder = newOrder;
		draggedIndex = index;
	};

	const handleDragEnd = () => {
		draggedIndex = null;
	};

	const capitalize = (str: string) => (str ? str[0].toUpperCase() + str.slice(1) : '');

	const getMetricName = (metricId: string): string => {
		const metric = metrics.find((m) => m.id === metricId);
		if (!metric) return metricId;

		if (metric.name) {
			return metric.name;
		}

		// Generate default title from metric properties
		const parts = [
			capitalize(metric.granularity.toLowerCase()),
			aggregateFunctionDisplay[metric.aggregate]
		];

		if (metric.aggregate !== 'NumberOfActivities') {
			parts.push(metric.metric.toLowerCase());
		}

		return parts.join(' ');
	};
</script>

<dialog class="modal" bind:this={dialog}>
	<div class="modal-box">
		<h3 class="mb-4 text-lg font-bold">Reorder Training Metrics</h3>

		{#if isLoadingOrder}
			<div class="flex justify-center py-8">
				<span class="loading loading-lg loading-spinner"></span>
			</div>
		{:else if metricsOrder.length === 0}
			<div class="py-8 text-center text-sm italic opacity-70">No metrics available.</div>
		{:else}
			<p class="mb-4 text-sm opacity-70">Drag and drop to reorder metrics.</p>
			<div role="list" class="space-y-2" aria-label="Training metrics list">
				{#each metricsOrder as metricId, index (metricId)}
					<div
						role="listitem"
						draggable="true"
						ondragstart={() => handleDragStart(index)}
						ondragover={(e) => handleDragOver(e, index)}
						ondragend={handleDragEnd}
						aria-label={`${getMetricName(metricId)}, position ${index + 1} of ${metricsOrder.length}`}
						class="flex cursor-move items-center gap-3 rounded-box bg-base-200 p-3 transition-colors hover:bg-base-300"
						class:opacity-50={draggedIndex === index}
					>
						<span class="text-lg" aria-hidden="true">☰</span>
						<span class="flex-1">{getMetricName(metricId)}</span>
						<span class="badge badge-sm">{index + 1}</span>
					</div>
				{/each}
			</div>
		{/if}

		<div class="modal-action">
			<button class="btn" onclick={() => dialog.close()} disabled={isSavingOrder}> Cancel </button>
			<button
				class="btn btn-primary"
				onclick={saveMetricsOrdering}
				disabled={isSavingOrder || isLoadingOrder || metricsOrder.length === 0}
			>
				{#if isSavingOrder}
					<span class="loading loading-sm loading-spinner"></span>
					Saving...
				{:else}
					Save Order
				{/if}
			</button>
		</div>
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
