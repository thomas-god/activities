<script lang="ts">
	import CreateTrainingPeriod from '$ui/navigation/internal/CreateTrainingPeriod.svelte';
	import Navbar from './Navbar.svelte';

	let {
		invalidateTrainingPeriods: invalidateTrainingPeriods
	}: { invalidateTrainingPeriods: () => void } = $props();

	let createTrainingPeriodDialog: HTMLDialogElement;

	const createTrainingPeriodCallback = () => {
		createTrainingPeriodDialog.close();
		invalidateTrainingPeriods();
	};

	const ctas = [
		{ label: 'New training period', onClick: () => createTrainingPeriodDialog.showModal() }
	];
</script>

<Navbar {ctas} />

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
