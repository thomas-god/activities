<script lang="ts">
	import CreateTrainingPeriod from '$components/organisms/CreateTrainingPeriod.svelte';
	import Navbar from './Navbar.svelte';

	let {
		invalidateTrainingPeriods: invalidateTrainingPeriods
	}: { invalidateTrainingPeriods: () => void } = $props();

	let createTrainingPeriodDialog: HTMLDialogElement;

	const createTrainingPeriodCallback = () => {
		createTrainingPeriodDialog.close();
		invalidateTrainingPeriods();
	};
</script>

{#snippet cta()}
	<div class="flex w-full flex-row justify-center gap-2 min-[750px]:justify-end">
		<button
			class="btn w-46 rounded-lg btn-sm btn-primary sm:btn-md"
			onclick={() => createTrainingPeriodDialog.showModal()}>+ New training period</button
		>
	</div>
{/snippet}

<Navbar {cta} />

<dialog class="modal" bind:this={createTrainingPeriodDialog}>
	<div class="modal-box max-w-3xl">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<CreateTrainingPeriod callback={createTrainingPeriodCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
