<script lang="ts">
	import CreateTrainingMetricFromTemplate from '$components/pages/CreateTrainingMetricFromTemplate.svelte';
	import Navbar from './Navbar.svelte';

	let { invalidateTrainingMetrics }: { invalidateTrainingMetrics: () => void } = $props();

	let createTrainingMetricDialog: HTMLDialogElement;

	const createTrainingMetricCallback = () => {
		createTrainingMetricDialog.close();
		invalidateTrainingMetrics();
	};

	// To prevent the form from loading when the dialog is initialized but hidden
	let showForm = $state(false);
</script>

{#snippet cta()}
	<div class="flex w-full flex-row justify-center gap-2 min-[750px]:justify-end">
		<button
			class="btn w-46 rounded-lg btn-sm btn-primary sm:btn-md"
			onclick={() => {
				showForm = true;
				createTrainingMetricDialog.showModal();
			}}>+ New training metric</button
		>
	</div>
{/snippet}

<Navbar {cta} />

<dialog class="modal" bind:this={createTrainingMetricDialog}>
	<div class="modal-box max-w-3xl">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		{#if showForm}
			<CreateTrainingMetricFromTemplate callback={createTrainingMetricCallback} />
		{/if}
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
