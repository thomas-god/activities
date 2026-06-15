<script lang="ts">
	import CreateTrainingMetricFromTemplate from '$components/pages/CreateTrainingMetricFromTemplate.svelte';
	import Navbar from './Navbar.svelte';

	let { invalidateTrainingMetrics }: { invalidateTrainingMetrics: () => void } = $props();

	let createTrainingMetricDialog: HTMLDialogElement;

	const createTrainingMetricCallback = () => {
		createTrainingMetricDialog.close();
		invalidateTrainingMetrics();
	};
</script>

{#snippet cta()}
	<div class="flex w-full flex-row justify-center gap-2 min-[750px]:justify-end">
		<button
			class="btn w-46 rounded-lg btn-sm btn-primary sm:btn-md"
			onclick={() => createTrainingMetricDialog.showModal()}>+ New training metric</button
		>
	</div>
{/snippet}

<Navbar {cta} />

<dialog class="modal" bind:this={createTrainingMetricDialog}>
	<div class="modal-box max-w-3xl">
		<CreateTrainingMetricFromTemplate callback={createTrainingMetricCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
