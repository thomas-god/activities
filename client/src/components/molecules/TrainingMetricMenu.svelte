<script lang="ts">
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';

	export type MetricProps = {
		id: string;
		name: string | null;
		scope: 'local' | 'global';
	};

	let {
		metric,
		onUpdate,
		onDelete
	}: { metric: MetricProps; onUpdate: () => void; onDelete: () => void } = $props();

	let showDeleteModal = $state(false);
	let isUpdating = $state(false);
	let editedName = $state(metric.name || '');
	let editNameDialog: HTMLDialogElement;
	let makeMetricGlobalDialog: HTMLDialogElement;

	const deleteMetricCallback = async (): Promise<void> => {
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric.id}`, {
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

	async function handleUpdateName() {
		if (!editedName.trim()) {
			alert('Name cannot be empty');
			return;
		}

		isUpdating = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric.id}`, {
				method: 'PATCH',
				credentials: 'include',
				mode: 'cors',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ name: editedName.trim() })
			});

			if (response.ok) {
				editNameDialog.close();
				onUpdate();
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

	const makeMetricGlobal = async (): Promise<void> => {
		const body = JSON.stringify({ scope: { type: 'global' } });
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric.id}`, {
			method: 'PATCH',
			credentials: 'include',
			mode: 'cors',
			headers: {
				'Content-Type': 'application/json'
			},
			body
		});

		if (res.status === 401) {
			goto('/login');
		}
		makeMetricGlobalDialog.close();
	};
</script>

<div class="dropdown dropdown-end">
	<div tabindex="0" role="button" class="btn btn-square btn-ghost btn-xs">⋮</div>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<ul tabindex="0" class="dropdown-content menu z-[1] w-52 rounded-box bg-base-100 p-2 shadow">
		<li>
			<button onclick={() => editNameDialog.show()}>
				<span>✏️</span>
				<span>Edit name</span>
			</button>
		</li>
		{#if metric.scope === 'local'}
			<li>
				<button onclick={() => makeMetricGlobalDialog.show()}>
					<span>🌐</span>
					<span>Make metric global</span>
				</button>
			</li>
		{/if}
		<li>
			<button onclick={() => (showDeleteModal = true)} class="text-error">
				<span>🗑️</span>
				<span>Delete</span>
			</button>
		</li>
	</ul>
</div>

<!-- Edit name modal -->
<dialog class="modal" bind:this={editNameDialog}>
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
			<button class="btn" onclick={() => editNameDialog.close()} disabled={isUpdating}>
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
		<button>close</button>
	</form>
</dialog>

<!-- Make metric global modal -->
<dialog class="modal" bind:this={makeMetricGlobalDialog}>
	<div class="modal-box">
		<h3 class="text-lg font-bold">Make training metric global</h3>
		<div class="py-4">
			<span> This will make the metric </span>
			<span class="italic">{metric.name}</span>
			<span>available to all other training periods outside this one</span>
		</div>
		<div class="modal-action">
			<button class="btn" onclick={() => makeMetricGlobalDialog.close()} disabled={isUpdating}>
				Cancel
			</button>
			<button class="btn btn-primary" onclick={makeMetricGlobal} disabled={isUpdating}>
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
		<button>close</button>
	</form>
</dialog>

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Training Metric"
	description="Are you sure you want to delete this training metric?"
	itemPreview={metric.name || 'Unnamed metric'}
	warning={metric.scope === 'global'
		? 'This metric is defined globally, deleting it will remove it from other training metrics.'
		: undefined}
	onConfirm={deleteMetricCallback}
/>
