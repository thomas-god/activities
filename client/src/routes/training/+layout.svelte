<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import CreateTrainingMetric from '$components/organisms/CreateTrainingMetric.svelte';
	import CreateTrainingPeriod from '$components/organisms/CreateTrainingPeriod.svelte';
	import CreateTrainingNote from '$components/organisms/CreateTrainingNote.svelte';

	let { children } = $props();

	const isSelected = (targetPath: string) => {
		return page.url.pathname === targetPath ? 'tab-active' : '';
	};

	const createTrainingMetricCallback = () => {
		createTrainingMetricDialog.close();
		invalidate('app:activities');
	};

	const createTrainingPeriodCallback = () => {
		createTrainingPeriodDialog.close();
	};

	const createTrainingNoteCallback = () => {
		createTrainingNoteDialog.close();
		invalidate('app:training-notes');
	};

	let createTrainingMetricDialog: HTMLDialogElement;
	let createTrainingPeriodDialog: HTMLDialogElement;
	let createTrainingNoteDialog: HTMLDialogElement;
</script>

<div class="@container">
	<div
		class="my-4 flex flex-col flex-wrap items-stretch gap-2 rounded-box bg-base-100 p-2 @min-[450px]:flex-row @min-[450px]:items-center @min-[450px]:justify-between"
	>
		<div role="tablist" class="tabs tabs-box px-3 font-semibold">
			<a role="tab" class={`tab  ${isSelected('/training/metrics')}`} href="/training/metrics"
				>Metrics</a
			>
			<a role="tab" class={`tab  ${isSelected('/training/periods')}`} href="/training/periods"
				>Periods</a
			>
			<a role="tab" class={`tab  ${isSelected('/training/notes')}`} href="/training/notes">Notes</a>
		</div>

		{#if page.url.pathname === '/training/metrics'}
			<div class="btn-create">
				<button
					class="btn w-full rounded-lg btn-primary"
					onclick={() => createTrainingMetricDialog.showModal()}>+ New training metric</button
				>
			</div>
		{:else if page.url.pathname === '/training/periods'}
			<div class="btn-create">
				<button
					class="btn w-full rounded-lg btn-primary"
					onclick={() => createTrainingPeriodDialog.showModal()}>+ New training period</button
				>
			</div>
		{:else if page.url.pathname === '/training/notes'}
			<div class="btn-create">
				<button
					class="btn w-full rounded-lg btn-primary"
					onclick={() => createTrainingNoteDialog.showModal()}>+ New note</button
				>
			</div>
		{/if}
	</div>
</div>

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

<dialog class="modal" id="create-training-note-modal" bind:this={createTrainingNoteDialog}>
	<div class="modal-box">
		<CreateTrainingNote callback={createTrainingNoteCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

<div>
	{@render children?.()}
</div>

<style>
	@container (width <= var(--container-3xs)) {
		.btn-create {
			flex-grow: 1;
		}
	}
</style>
