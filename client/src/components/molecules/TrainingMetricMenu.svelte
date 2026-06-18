<script lang="ts">
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import UpdateTrainingMetricFrom from '$components/pages/UpdateTrainingMetricFrom.svelte';
	import type { TrainingMetric } from '$lib/api';
	import { none } from '$lib/Options';

	let {
		metric,
		name,
		id,
		scope,
		onUpdate,
		onDelete
	}: {
		metric: TrainingMetric;
		id: string;
		name: string | null;
		scope: 'global' | 'local';
		onUpdate: () => void;
		onDelete: () => void;
	} = $props();

	let showDeleteModal = $state(false);
	let editMetricDialog: HTMLDialogElement;

	const deleteMetricCallback = async (): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${id}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (res.status === 401) {
			goto('/login');
		}
		showDeleteModal = false;
		onDelete();
	};
</script>

<div class="dropdown dropdown-end">
	<button tabindex="0" class="btn px-0.5 btn-xs" aria-label="Metric options">
		<img src="/icons/menu.svg" class="inline h-7 w-7" alt="Menu icon" />
	</button>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<ul tabindex="0" class="dropdown-content menu z-[1] w-40 rounded-box bg-base-100 p-2 shadow">
		<li>
			<button onclick={() => editMetricDialog.show()}>
				<img src="/icons/edit.svg" alt="Edit icon" class="h-6 w-6" /> Edit metric
			</button>
		</li>
		<li>
			<button onclick={() => (showDeleteModal = true)} class="text-error">
				<img src="/icons/delete.svg" alt="Delete icon" class="h-6 w-6" /> Delete
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
		<UpdateTrainingMetricFrom
			initialMetric={metric}
			callback={onUpdate}
			existingSportsConstraints={none()}
		/>
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
	itemPreview={name || 'Unnamed metric'}
	warning={scope === 'global'
		? 'This metric is defined globally, deleting it will remove it from other training metrics.'
		: undefined}
	onConfirm={deleteMetricCallback}
/>
