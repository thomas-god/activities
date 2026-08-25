<script lang="ts">
	import ActivitiesUploader from '$ui/navigation/internal/ActivitiesUploader.svelte';
	import CreateTrainingNote from '$ui/navigation/internal/CreateTrainingNote.svelte';
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

	const ctas = [
		{ label: 'Add activities', onClick: () => activitiesUploadDialog.showModal() },
		{ label: 'New note', onClick: () => newTrainingNoteDialog.showModal() }
	];
</script>

<Navbar {ctas} />

<dialog class="modal" bind:this={activitiesUploadDialog}>
	<div class="modal-box">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<ActivitiesUploader {activitiesUploadedCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

<dialog class="modal" bind:this={newTrainingNoteDialog}>
	<div class="modal-box">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		<CreateTrainingNote callback={newTrainingNoteCallback} />
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
