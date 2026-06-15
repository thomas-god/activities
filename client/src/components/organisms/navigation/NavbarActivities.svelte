<script lang="ts">
	import ActivitiesUploader from '$components/organisms/ActivitiesUploader.svelte';
	import CreateTrainingNote from '$components/organisms/CreateTrainingNote.svelte';
	import Navbar from './Navbar.svelte';

	let {
		invalidateActivities,
		invalidateTrainingNotes
	}: { invalidateActivities: () => void; invalidateTrainingNotes: () => void } = $props();

	let activitiesUploadDialog: HTMLDialogElement;
	let newTrainingNoteDialog: HTMLDialogElement;

	const activitiesUploadedCallback = () => {
		invalidateActivities();
	};

	const newTrainingNoteCallback = () => {
		newTrainingNoteDialog.close();
		invalidateTrainingNotes();
	};
</script>

{#snippet cta()}
	<div class="flex w-full flex-row justify-center gap-2 min-[750px]:justify-end">
		<button
			class="btn w-36 rounded-lg btn-sm btn-primary sm:btn-md"
			onclick={() => activitiesUploadDialog.showModal()}>+ Add activities</button
		>
		<button
			class="btn w-36 rounded-lg btn-sm btn-primary sm:btn-md"
			onclick={() => newTrainingNoteDialog.showModal()}>+ New note</button
		>
	</div>
{/snippet}

<Navbar {cta} />

<dialog class="modal" bind:this={activitiesUploadDialog}>
	<div class="modal-box">
		<ActivitiesUploader {activitiesUploadedCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

<dialog class="modal" bind:this={newTrainingNoteDialog}>
	<div class="modal-box">
		<CreateTrainingNote callback={newTrainingNoteCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
