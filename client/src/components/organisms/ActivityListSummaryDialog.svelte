<script lang="ts">
	import type { ActivityListSummaryItems } from '$lib/api';
	import { toTitleCase } from '$lib/utils';

	interface SummaryItem {
		type: 'metric' | 'rpe' | 'workoutType';
		value?: string; // only for metric type
		displayName: string;
		key: string; // unique identifier for the item
	}

	interface Props {
		defaultMetrics: string[];
		currentPreference: ActivityListSummaryItems;
		isOpen: boolean;
		onSave: (items: ActivityListSummaryItems) => Promise<boolean>;
	}

	let { defaultMetrics, currentPreference, onSave, isOpen = $bindable() }: Props = $props();

	let draggedIndex = $state<number | null>(null);

	const RPE_ITEM: SummaryItem = {
		type: 'rpe',
		displayName: 'RPE',
		key: 'rpe'
	};

	const WORKOUT_TYPE_ITEM: SummaryItem = {
		type: 'workoutType',
		displayName: 'Workout Type',
		key: 'workoutType'
	};

	// All possible items derived from default metrics and rpe/workout type
	const allItems = $derived<SummaryItem[]>([
		RPE_ITEM,
		WORKOUT_TYPE_ITEM,
		...defaultMetrics.map((metric) => ({
			type: 'metric' as const,
			value: metric,
			displayName: toTitleCase(metric),
			key: `metric:${metric}`
		}))
	]);

	let selectedItems = $derived<SummaryItem[]>(
		currentPreference
			.map((item) => {
				if (item.type === 'rpe') return RPE_ITEM;
				if (item.type === 'workoutType') return WORKOUT_TYPE_ITEM;
				if (item.type === 'metric' && item.value) {
					return allItems.find((i) => i.type === 'metric' && i.value === item.value) ?? null;
				}
				return null;
			})
			.filter((item): item is SummaryItem => item !== null)
	);

	// Available items = all items minus selected items
	const availableItems = $derived<SummaryItem[]>(
		allItems.filter((item) => !selectedItems.some((s) => s.key === item.key))
	);

	const saveSelection = async () => {
		const items: ActivityListSummaryItems = selectedItems.map((item) => {
			if (item.type === 'metric' && item.value) {
				return { type: 'metric' as const, value: item.value };
			}
			return { type: item.type as 'rpe' | 'workoutType' };
		});
		await onSave(items);
		isOpen = false;
	};

	const addItem = (item: SummaryItem) => {
		selectedItems = [...selectedItems, item];
	};

	const removeItem = (item: SummaryItem) => {
		selectedItems = selectedItems.filter((i) => i.key !== item.key);
	};

	// Drag and drop handlers for selected items
	const handleDragStart = (index: number) => {
		draggedIndex = index;
	};

	const handleDragOver = (e: DragEvent, index: number) => {
		e.preventDefault();
		if (draggedIndex === null || draggedIndex === index) return;

		const newOrder = [...selectedItems];
		const [draggedItem] = newOrder.splice(draggedIndex, 1);
		newOrder.splice(index, 0, draggedItem);
		selectedItems = newOrder;
		draggedIndex = index;
	};

	const handleDragEnd = () => {
		draggedIndex = null;
	};

	const moveItemUp = (currentIndex: number) => {
		if (currentIndex === 0) return;

		const newOrder = [...selectedItems];
		const [movedItem] = newOrder.splice(currentIndex, 1);
		newOrder.splice(currentIndex - 1, 0, movedItem);
		selectedItems = newOrder;
	};

	const moveItemDown = (currentIndex: number) => {
		if (currentIndex === selectedItems.length - 1) return;

		const newOrder = [...selectedItems];
		const [movedItem] = newOrder.splice(currentIndex, 1);
		newOrder.splice(currentIndex + 1, 0, movedItem);
		selectedItems = newOrder;
	};
</script>

{#if isOpen}
	<dialog class="modal" open>
		<div class="modal-box max-w-4xl">
			<form method="dialog">
				<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
			</form>
			<h3 class="mb-4 text-lg font-bold">Configure Activity List</h3>

			<p class="mb-4 text-sm opacity-70">
				Select activity's statistics to show in the activity list and arrange them in your preferred
				order.
			</p>

			<div class="grid gap-4">
				<!-- Selected Items (with ordering) -->
				<div>
					<h4 class="mb-2 text-sm font-semibold">Selected statistics (in order)</h4>
					<div
						role="list"
						class="max-h-75 space-y-2 overflow-scroll rounded-box border border-base-300 bg-base-100 p-3"
						aria-label="Selected items"
					>
						{#if selectedItems.length === 0}
							<div class="py-8 text-center text-sm italic opacity-70">No items selected</div>
						{:else}
							{#each selectedItems as item, index (item.key)}
								<div
									role="listitem"
									draggable="true"
									ondragstart={() => handleDragStart(index)}
									ondragover={(e) => handleDragOver(e, index)}
									ondragend={handleDragEnd}
									aria-label={`${item.displayName}, position ${index + 1} of ${selectedItems.length}`}
									class="flex cursor-move items-center gap-1 rounded-box bg-base-200 p-2 transition-colors hover:bg-base-300"
									class:opacity-50={draggedIndex === index}
								>
									<!-- Drag handle for desktop -->
									<img
										src="/icons/list.svg"
										class="h-4 w-4 pointer-coarse:hidden"
										aria-hidden="true"
										alt="Drag handle"
									/>

									<!-- Up/down buttons for mobile -->
									<button
										class="btn btn-ghost px-0 btn-xs pointer-fine:hidden"
										onclick={() => moveItemUp(index)}
										disabled={index === 0}
										aria-label={`Move ${item.displayName} up`}
									>
										<img src="/icons/up.svg" class="h-4 w-4" alt="Up arrow" />
									</button>
									<button
										class="btn btn-ghost px-0 btn-xs pointer-fine:hidden"
										onclick={() => moveItemDown(index)}
										disabled={index === selectedItems.length - 1}
										aria-label={`Move ${item.displayName} down`}
									>
										<img src="/icons/down.svg" class="h-4 w-4" alt="Down arrow" />
									</button>

									<span class="flex-1 text-sm">{item.displayName}</span>
									<span class="badge badge-xs">{index + 1}</span>

									<!-- Remove button -->
									<button
										class="btn btn-ghost btn-xs"
										onclick={() => removeItem(item)}
										aria-label={`Remove ${item.displayName}`}
									>
										<img src="/icons/close.svg" class="h-4 w-4" alt="Remove icon" />
									</button>
								</div>
							{/each}
						{/if}
					</div>
				</div>

				<!-- Available Items -->
				<div>
					<h4 class="mb-2 text-sm font-semibold">Available statistics</h4>
					<div
						role="list"
						class="max-h-75 space-y-2 overflow-scroll rounded-box border border-base-300 bg-base-100 p-3"
						aria-label="Available items"
					>
						{#if availableItems.length === 0}
							<div class="py-8 text-center text-sm italic opacity-70">All items are selected</div>
						{:else}
							{#each availableItems as item (item.key)}
								<div
									role="listitem"
									class="flex items-center justify-between rounded-box bg-base-200 p-2 transition-colors hover:bg-base-300"
								>
									<span class="text-sm">{item.displayName}</span>
									<button
										class="btn btn-primary btn-sm"
										onclick={() => addItem(item)}
										aria-label={`Add ${item.displayName}`}
									>
										<img src="/icons/plus.svg" class="h-4 w-4" alt="Add icon" />
									</button>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			</div>

			<div class="modal-action">
				<button class="btn" onclick={() => (isOpen = false)}> Cancel </button>
				<button
					class="btn btn-primary"
					onclick={saveSelection}
					disabled={selectedItems.length === 0}
				>
					Save
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>
{/if}
