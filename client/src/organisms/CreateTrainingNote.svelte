<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { createTrainingNote } from '$lib/api/training';
	import { dayjs } from '$lib/duration';

	let { callback }: { callback: () => void } = $props();

	let title = $state<string | null>(null);
	let content = $state('');
	let date = $state(dayjs().format('YYYY-MM-DD'));
	let requestPending = $state(false);
	let errorMessage = $state('');

	let missingInformation = $derived(content.trim() === '');

	const handleCreateNote = async () => {
		if (missingInformation || requestPending) return;

		requestPending = true;
		errorMessage = '';

		try {
			await createTrainingNote(content.trim(), title, date);
			invalidate('app:training-notes');
			content = '';
			title = null;
			date = dayjs().format('YYYY-MM-DD');
			callback();
		} catch (error) {
			errorMessage = 'An error occurred. Please try again.';
		} finally {
			requestPending = false;
		}
	};
</script>

<div class="text-sm">
	<fieldset class="fieldset rounded-box bg-base-100 p-2">
		<label class="label" for="note-title">Title (optional)</label>
		<input
			type="text"
			placeholder="Optional title"
			class="input"
			id="note-title"
			bind:value={title}
		/>

		<label class="label" for="note-date">Date</label>
		<input type="date" class="input" id="note-date" bind:value={date} />

		<label class="label" for="note-content">Note content</label>
		<textarea
			id="note-content"
			class="textarea-bordered textarea"
			placeholder="Write your training note here..."
			rows="8"
			bind:value={content}
		></textarea>

		{#if errorMessage}
			<p class="label text-error">{errorMessage}</p>
		{/if}

		<div class="mt-2 flex flex-row items-center justify-center gap-2">
			<button
				class="btn flex-1 btn-sm btn-primary"
				onclick={handleCreateNote}
				disabled={missingInformation || requestPending}
			>
				{#if requestPending}
					<span class="loading loading-xs loading-spinner"></span>
					Creating...
				{:else}
					Create note
				{/if}
			</button>
		</div>
	</fieldset>
</div>
