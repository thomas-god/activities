<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import ActivitiesUploader from '$components/organisms/ActivitiesUploader.svelte';
	import CreateTrainingNote from '$components/organisms/CreateTrainingNote.svelte';
	import CreateTrainingMetric from '$components/organisms/CreateTrainingMetric.svelte';
	import CreateTrainingPeriod from '$components/organisms/CreateTrainingPeriod.svelte';

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
	const createTrainingMetricCallback = () => {
		createTrainingMetricDialog.close();
		invalidate('app:activities');
	};

	const createTrainingPeriodCallback = () => {
		createTrainingPeriodDialog.close();
	};

	let activitiesUploadDialog: HTMLDialogElement;
	let newTrainingNoteDialog: HTMLDialogElement;
	let createTrainingMetricDialog: HTMLDialogElement;
	let createTrainingPeriodDialog: HTMLDialogElement;
</script>

<div class="layout_container">
	<div class="flex flex-col justify-between gap-3 min-[750px]:flex-row min-[750px]:items-center">
		<div class="flex gap-3 sm:gap-6">
			<a class={`btn px-2 text-lg font-bold btn-ghost sm:text-xl ${classExactPath('/')}`} href="/"
				>Activities</a
			>
			<a
				class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classExactPath('/history')}`}
				href="/history">History</a
			>
			<a
				class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classPathStartWith('/training/metrics')}`}
				href="/training/metrics">Metrics</a
			>
			<a
				class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classPathStartWith('/training/period')}`}
				href="/training/periods">Periods</a
			>
		</div>
		{#if ['/', '/history'].includes(page.url.pathname)}
			<div class="flex w-full flex-row justify-center gap-2 min-[750px]:justify-end">
				<button
					class="btn w-32 rounded-lg btn-sm btn-primary sm:w-36 sm:btn-md"
					onclick={() => activitiesUploadDialog.showModal()}>+ Add activities</button
				>
				<button
					class="btn w-32 rounded-lg btn-sm btn-primary sm:w-36 sm:btn-md"
					onclick={() => newTrainingNoteDialog.showModal()}>+ New note</button
				>
			</div>
		{:else if page.url.pathname === '/training/metrics'}
			<div class="btn-create">
				<button
					class="btn w-full rounded-lg btn-primary"
					onclick={() => createTrainingMetricDialog.showModal()}>+ New training metric</button
				>
			</div>
		{:else if page.url.pathname.startsWith('/training/period')}
			<div class="btn-create">
				<button
					class="btn w-full rounded-lg btn-primary"
					onclick={() => createTrainingPeriodDialog.showModal()}>+ New training period</button
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

	<dialog class="modal" id="create-training-metric-modal" bind:this={createTrainingMetricDialog}>
		<div class="modal-box max-w-3xl">
			<CreateTrainingMetric callback={createTrainingMetricCallback} />
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>

	<dialog class="modal" id="create-training-period-modal" bind:this={createTrainingPeriodDialog}>
		<div class="modal-box">
			<CreateTrainingPeriod callback={createTrainingPeriodCallback} />
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
