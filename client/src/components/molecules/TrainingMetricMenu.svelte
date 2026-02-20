<script lang="ts">
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { goto } from '$app/navigation';
	import EditButton from '$components/atoms/EditButton.svelte';
	import DeleteButton from '$components/atoms/DeleteButton.svelte';

	let {
		name,
		id,
		scope,
		onUpdate,
		onDelete
	}: {
		id: string;
		name: string | null;
		scope: 'global' | 'local';
		onUpdate: () => void;
		onDelete: () => void;
	} = $props();

	let showDeleteModal = $state(false);
	let isUpdating = $state(false);
	let editedName = $state(name || '');
	let editNameDialog: HTMLDialogElement;
	let makeMetricGlobalDialog: HTMLDialogElement;

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

	async function handleUpdateName() {
		if (!editedName.trim()) {
			alert('Name cannot be empty');
			return;
		}

		isUpdating = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${id}`, {
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
		const res = await fetch(`${PUBLIC_APP_URL}/api/training/metric/${id}`, {
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
	<div tabindex="0" role="button" class="btn btn-square btn-ghost btn-xs">
		<img src="/icons/menu.svg" alt="Menu icon" class="h-5 w-5" />
	</div>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<ul
		tabindex="0"
		class="dropdown-content menu z-[1] flex w-52 flex-col items-start rounded-box bg-base-100 p-2 shadow"
	>
		<li>
			<EditButton callback={() => editNameDialog.show()} text="Edit name" size="normal" />
		</li>
		{#if scope === 'local'}
			<li>
				<button onclick={() => makeMetricGlobalDialog.show()} class="btn btn-ghost">
					<img src="/icons/globe.svg" class="h-4 w-4" alt="Globe icon" />
					<span>Make metric global</span>
				</button>
			</li>
		{/if}
		<li>
			<DeleteButton callback={() => (showDeleteModal = true)} text="Delete" />
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
			<span class="italic">{name}</span>
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
	itemPreview={name || 'Unnamed metric'}
	warning={scope === 'global'
		? 'This metric is defined globally, deleting it will remove it from other training metrics.'
		: undefined}
	onConfirm={deleteMetricCallback}
/>
