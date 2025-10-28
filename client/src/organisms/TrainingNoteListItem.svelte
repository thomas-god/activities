<script lang="ts">
	import { dayjs } from '$lib/duration';
	import type { TrainingNote } from '$lib/api/training';

	interface Props {
		note: TrainingNote;
		onSave: (content: string, date: string) => void;
		onDelete: () => void;
	}

	let { note, onSave, onDelete }: Props = $props();

	let isEditing = $state(false);
	let isExpanded = $state(false);
	let showDeleteModal = $state(false);
	let isDeleting = $state(false);
	let editContent = $state('');
	let editDate = $state('');

	const startEdit = () => {
		isEditing = true;
		editContent = note.content;
		editDate = note.date;
	};

	const cancelEdit = () => {
		isEditing = false;
		editContent = '';
		editDate = '';
	};

	const saveEdit = () => {
		if (editContent.trim() === '') return;
		onSave(editContent.trim(), editDate);
		isEditing = false;
		editContent = '';
		editDate = '';
	};

	const toggleExpand = () => {
		isExpanded = !isExpanded;
	};

	const confirmDelete = () => {
		showDeleteModal = true;
	};

	const cancelDelete = () => {
		showDeleteModal = false;
		isDeleting = false;
	};

	const handleDelete = async () => {
		isDeleting = true;
		await onDelete();
		isDeleting = false;
		showDeleteModal = false;
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

<div class="border-b border-base-200 p-4">
	<div class="mb-2 flex items-center justify-between">
		<div class="text-xs font-light opacity-70">
			<div class="text-xs opacity-60">
				{dayjs(note.date).format('MMM D, YYYY')}
			</div>
		</div>
		{#if !isEditing}
			<div class="flex gap-2">
				<button class="btn btn-ghost btn-xs" onclick={startEdit}> ✏️ Edit </button>
				<button class="btn btn-ghost btn-xs" onclick={confirmDelete}> 🗑️ Delete </button>
			</div>
		{/if}
	</div>

	{#if isEditing}
		<div class="flex flex-col gap-2">
			<label class="floating-label">
				<input type="date" class="input" bind:value={editDate} />
				<span>Date</span>
			</label>

			<textarea
				class="textarea-bordered textarea w-full"
				rows="6"
				placeholder="Content"
				bind:value={editContent}
			></textarea>
			<div class="flex justify-end gap-2">
				<button class="btn btn-ghost btn-sm" onclick={cancelEdit}>Cancel</button>
				<button
					class="btn btn-sm btn-primary"
					onclick={saveEdit}
					disabled={editContent.trim() === ''}
				>
					Save
				</button>
			</div>
		</div>
	{:else}
		<div class="text-sm whitespace-pre-wrap">{getDisplayContent(note.content)}</div>
		{#if shouldShowExpandButton(note.content)}
			<button class="btn mt-2 btn-ghost btn-xs" onclick={toggleExpand}>
				{isExpanded ? '▲ Show less' : '▼ Show more'}
			</button>
		{/if}
	{/if}
</div>

<!-- Delete confirmation modal -->
{#if showDeleteModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Delete Training Note</h3>
			<p class="py-4">
				Are you sure you want to delete this note?
				<br />
				<span class="mt-2 block text-sm italic opacity-70">
					<span class="line-clamp-3">
						{note.content.slice(0, 75)}{note.content.length > 75 ? '...' : ''}
					</span>
				</span>
				<br />
				<strong>This action cannot be undone.</strong>
			</p>
			<div class="modal-action">
				<button class="btn" onclick={cancelDelete} disabled={isDeleting}> Cancel </button>
				<button class="btn btn-error" onclick={handleDelete} disabled={isDeleting}>
					{#if isDeleting}
						<span class="loading loading-sm loading-spinner"></span>
						Deleting...
					{:else}
						Delete
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={cancelDelete}>close</button>
		</form>
	</dialog>
{/if}
