<script lang="ts">
	import { dayjs } from '$lib/duration';
	import type { TrainingNote } from '$lib/api/training';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';

	interface Props {
		note: TrainingNote;
		onSave: (content: string, date: string) => void;
		onDelete: () => void;
	}

	let { note, onSave, onDelete }: Props = $props();

	let isExpanded = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let isSaving = $state(false);
	let editContent = $state('');
	let editDate = $state('');

	const startEdit = () => {
		showEditModal = true;
		editContent = note.content;
		editDate = note.date;
	};

	const cancelEdit = () => {
		showEditModal = false;
		editContent = '';
		editDate = '';
	};

	const saveEdit = async () => {
		if (editContent.trim() === '') return;
		isSaving = true;
		await onSave(editContent.trim(), editDate);
		isSaving = false;
		showEditModal = false;
		editContent = '';
		editDate = '';
	};

	const toggleExpand = () => {
		isExpanded = !isExpanded;
	};

	const confirmDelete = () => {
		showDeleteModal = true;
	};

	const handleDelete = async () => {
		await onDelete();
	};

	const MAX_CHARS_COLLAPSED = 300; // Maximum characters to show when collapsed
	const MAX_LINES_COLLAPSED = 3; // Maximum lines to show when collapsed

	const getDisplayContent = (content: string) => {
		if (isExpanded) {
			return content;
		}

		const lines = content.split('\n');
		const isTooManyLines = lines.length > MAX_LINES_COLLAPSED;
		const isTooLong = content.length > MAX_CHARS_COLLAPSED;

		if (!isTooManyLines && !isTooLong) {
			return content;
		}

		// If too many lines, show first N lines
		if (isTooManyLines) {
			return lines.slice(0, MAX_LINES_COLLAPSED).join('\n');
		}

		// If too long but not too many lines, truncate at character limit
		return content.slice(0, MAX_CHARS_COLLAPSED) + '...';
	};

	const shouldShowExpandButton = (content: string) => {
		const lines = content.split('\n');
		return lines.length > MAX_LINES_COLLAPSED || content.length > MAX_CHARS_COLLAPSED;
	};
</script>

<div>
	<div class="mb-1 flex items-center gap-2">
		<div class="text-xs font-light opacity-70">
			<div class="text-xs opacity-60">
				{dayjs(note.date).format('MMM D, YYYY')}
			</div>
		</div>
		<div class="dropdown dropdown-end">
			<div tabindex="0" role="button" class="btn btn-square btn-ghost btn-xs">⋮</div>
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<ul tabindex="0" class="dropdown-content menu z-[1] w-32 rounded-box bg-base-100 p-2 shadow">
				<li>
					<button onclick={startEdit}>
						<span>✏️</span>
						<span>Edit</span>
					</button>
				</li>
				<li>
					<button onclick={confirmDelete} class="text-error">
						<span>🗑️</span>
						<span>Delete</span>
					</button>
				</li>
			</ul>
		</div>
	</div>

	<div class="text-sm whitespace-pre-wrap">{getDisplayContent(note.content)}</div>
	{#if shouldShowExpandButton(note.content)}
		<button class="btn mt-2 btn-ghost btn-xs" onclick={toggleExpand}>
			{isExpanded ? '▲ Show less' : '▼ Show more'}
		</button>
	{/if}
</div>

<!-- Edit modal -->
{#if showEditModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Note</h3>
			<div class="flex flex-col gap-4 py-4">
				<label class="floating-label">
					<input type="date" class="input" bind:value={editDate} disabled={isSaving} />
					<span>Date</span>
				</label>

				<textarea
					class="textarea-bordered textarea w-full"
					rows="8"
					placeholder="Content"
					bind:value={editContent}
					disabled={isSaving}
				></textarea>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={cancelEdit} disabled={isSaving}> Cancel </button>
				<button
					class="btn btn-primary"
					onclick={saveEdit}
					disabled={editContent.trim() === '' || isSaving}
				>
					{#if isSaving}
						<span class="loading loading-sm loading-spinner"></span>
						Saving...
					{:else}
						Save
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={cancelEdit}>close</button>
		</form>
	</dialog>
{/if}

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Training Note"
	description="Are you sure you want to delete this note?"
	itemPreview={note.content.slice(0, 75) + (note.content.length > 75 ? '...' : '')}
	onConfirm={handleDelete}
/>
