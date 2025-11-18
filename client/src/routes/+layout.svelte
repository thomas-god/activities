<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import ActivitiesUploader from '$components/organisms/ActivitiesUploader.svelte';
	import CreateTrainingNote from '$components/organisms/CreateTrainingNote.svelte';

	let { children } = $props();

	const classExactPath = (targetPath: string): string => {
		return page.url.pathname === targetPath ? 'active' : '';
	};

	const classPathStartWith = (targetPath: string): string => {
		return page.url.pathname.startsWith(targetPath) ? 'active' : '';
	};

	const activitiesUploadedCallback = () => {
		invalidate('app:activities');
	};

	const newTrainingNoteCallback = () => {
		newTrainingNoteDialog.close();
		invalidate('app:training-notes');
	};

	let activitiesUploadDialog: HTMLDialogElement;
	let newTrainingNoteDialog: HTMLDialogElement;
</script>

<div class="layout_container">
	<div class=" flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
		<div class="flex gap-6">
			<a class={`btn px-2 text-xl font-bold btn-ghost ${classExactPath('/')}`} href="/"
				>Activities</a
			>
			<a
				class={`btn px-2 text-lg font-medium btn-ghost ${classExactPath('/history')}`}
				href="/history">History</a
			>
			<a
				class={`btn px-2 text-lg font-medium btn-ghost ${classPathStartWith('/training')}`}
				href="/training/metrics">Training</a
			>
		</div>
		{#if page.url.pathname === '/'}
			<div class="flex w-full flex-row justify-center gap-2 sm:justify-end">
				<button
					class="btn w-32 rounded-lg btn-sm btn-primary sm:w-36 sm:btn-md"
					onclick={() => activitiesUploadDialog.showModal()}>+ Add activities</button
				>
				<button
					class="btn w-32 rounded-lg btn-sm btn-primary sm:w-36 sm:btn-md"
					onclick={() => newTrainingNoteDialog.showModal()}>+ New note</button
				>
			</div>
		{/if}
	</div>

	<dialog class="modal" id="activity-upload-modal" bind:this={activitiesUploadDialog}>
		<div class="modal-box">
			<ActivitiesUploader {activitiesUploadedCallback} />
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>

	<dialog class="modal" id="new-training-note-modal" bind:this={newTrainingNoteDialog}>
		<div class="modal-box">
			<CreateTrainingNote callback={newTrainingNoteCallback} />
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>

	{@render children?.()}
</div>

<style>
	.layout_container {
		max-width: 1350px;
		margin: 0 auto;
		padding: 12px 8px;

		@media (min-width: 640px) {
			padding: 20px;
		}
	}

	.active {
		border-bottom-color: var(--color-primary);
		border-bottom-width: 2px;
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
</style>
