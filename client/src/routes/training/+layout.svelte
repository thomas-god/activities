<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { page } from '$app/state';
	import CreateTrainingMetric from '../../organisms/CreateTrainingMetric.svelte';
	import CreateTrainingPeriod from '../../organisms/CreateTrainingPeriod.svelte';

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

	let createTrainingMetricDialog: HTMLDialogElement;
	let createTrainingPeriodDialog: HTMLDialogElement;
</script>

<div class="@container">
	<div
		class="my-4 flex flex-col flex-wrap items-stretch gap-2 rounded-box bg-base-100 p-2 @min-[400px]:flex-row @min-[400px]:items-center @min-[400px]:justify-between"
	>
		<div role="tablist" class="tabs tabs-box px-3 font-semibold">
			<a role="tab" class={`tab  ${isSelected('/training/metrics')}`} href="/training/metrics"
				>Metrics</a
			>
			<a role="tab" class={`tab  ${isSelected('/training/periods')}`} href="/training/periods"
				>Periods</a
			>
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
		{/if}
	</div>
</div>

<dialog class="modal" id="create-training-metric-modal" bind:this={createTrainingMetricDialog}>
	<div class="modal-box">
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

<div>
	{@render children?.()}
</div>

<style>
	@container (width <= var(--container-3xs)) {
		.btn-create {
			flex-grow: 1;
		}
		/* margin-left: auto; */
		/* margin-right: auto; */
	}
</style>
