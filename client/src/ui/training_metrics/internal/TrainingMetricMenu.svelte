<script lang="ts">
	import DeleteModal from '$ui/shared/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import TrainingMetricFormUpdate from '../TrainingMetricFormUpdate.svelte';
	import type { TrainingMetric } from '$lib/api';
	import { none } from '$lib/Options';
	import { resolve } from '$app/paths';
	import { Menu, Pencil, Trash2 } from '@lucide/svelte';

	let {
		metric,
		onUpdate,
		onDelete
	}: {
		metric: TrainingMetric;
		onUpdate: () => void;
		onDelete: () => void;
	} = $props();

	let showDeleteModal = $state(false);
	let editMetricDialog: HTMLDialogElement;

	// To prevent the form from loading when the dialog is initialized but hidden
	let showEditForm = $state(false);

	const deleteMetricCallback = async (): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric.id}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (res.status === 401) {
			goto(resolve('/login'));
		}
		showDeleteModal = false;
		onDelete();
	};
</script>

<div class="dropdown dropdown-end">
	<button tabindex="0" class="btn px-0.5 btn-xs" aria-label="Metric options">
		<Menu class="size-5" />
	</button>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<ul tabindex="0" class="menu dropdown-content z-1 w-40 rounded-box bg-base-100 p-2 shadow">
		<li>
			<button
				onclick={() => {
					showEditForm = true;
					editMetricDialog.show();
				}}
			>
				<Pencil class="size-4" />
				Edit metric
			</button>
		</li>
		<li>
			<button onclick={() => (showDeleteModal = true)} class="text-error">
				<Trash2 class="size-4" />
				Delete
			</button>
		</li>
	</ul>
</div>

<!-- Edit name modal -->
<dialog class="modal" bind:this={editMetricDialog}>
	<div class="modal-box max-w-2xl text-start">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		{#if showEditForm}
			<TrainingMetricFormUpdate
				initialMetric={metric}
				callback={onUpdate}
				existingSportsConstraints={none()}
			/>
		{/if}
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Training Metric"
	description="Are you sure you want to delete this training metric?"
	itemPreview={metric.name || 'Unnamed metric'}
	warning={metric.scope.type === 'global'
		? 'This metric is defined globally, deleting it will remove it from other training metrics.'
		: undefined}
	onConfirm={deleteMetricCallback}
/>
