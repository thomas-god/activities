<script lang="ts">
	import TrainingMetricFormCreate from '$ui/training_metrics/TrainingMetricFormCreate.svelte';
	import Navbar from './Navbar.svelte';

	let { invalidateTrainingMetrics }: { invalidateTrainingMetrics: () => void } = $props();

	let createTrainingMetricDialog: HTMLDialogElement;

	const createTrainingMetricCallback = () => {
		createTrainingMetricDialog.close();
		invalidateTrainingMetrics();
	};

	// To prevent the form from loading when the dialog is initialized but hidden
	let showForm = $state(false);

	const ctas = [
		{
			label: 'New training metric',
			onClick: () => {
				showForm = true;
				createTrainingMetricDialog.showModal();
			}
		}
	];
</script>

<Navbar {ctas} />

<dialog class="modal" bind:this={createTrainingMetricDialog}>
	<div class="modal-box max-w-3xl">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		{#if showForm}
			<TrainingMetricFormCreate callback={createTrainingMetricCallback} />
		{/if}
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
