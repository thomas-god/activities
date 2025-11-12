<script lang="ts">
	import { dayjs, formatDurationHoursMinutes } from '$lib/duration';
	import { getSportCategory, getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import { goto, invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import type { TrainingNote } from './+page';
	import ActivitiesListItem from '$components/organisms/ActivitiesListItem.svelte';
	import DeleteModal from '$components/molecules/DeleteModal.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';
	import TrainingNoteListItemCompact from '$components/organisms/TrainingNoteListItemCompact.svelte';

	let { data }: PageProps = $props();

	let showDeleteModal = $state(false);
	let showEditModal = $state(false);
	let isUpdating = $state(false);
	let editedName = $state('');
	let showEditNoteModal = $state(false);
	let editedNote = $state('');
	let isUpdatingNote = $state(false);

	async function handleDelete() {
		const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${period.id}`, {
			method: 'DELETE',
			credentials: 'include',
			mode: 'cors'
		});

		if (response.ok) {
			await goto('/training/periods');
		} else {
			throw new Error('Failed to delete training period');
		}
	}

	function openEditModal() {
		editedName = period.name;
		showEditModal = true;
	}

	function openEditNoteModal() {
		editedNote = period.note ?? '';
		showEditNoteModal = true;
	}

	async function handleUpdateNote() {
		isUpdatingNote = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${period.id}`, {
				method: 'PATCH',
				credentials: 'include',
				mode: 'cors',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ note: editedNote.trim() })
			});

			if (response.ok) {
				// Reload the page to show updated content
				window.location.reload();
			} else {
				const error = await response.json();
				alert(error.error || 'Failed to update training period note');
			}
		} catch (error) {
			alert('Error updating training period note');
			console.error(error);
		} finally {
			isUpdatingNote = false;
		}
	}

	async function handleUpdate() {
		if (!editedName.trim()) {
			alert('Name cannot be empty');
			return;
		}

		isUpdating = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${period.id}`, {
				method: 'PATCH',
				credentials: 'include',
				mode: 'cors',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ name: editedName.trim() })
			});

			if (response.ok) {
				// Reload the page to show updated content
				window.location.reload();
			} else {
				const error = await response.json();
				alert(error.error || 'Failed to update training period name');
			}
		} catch (error) {
			alert('Error updating training period name');
			console.error(error);
		} finally {
			isUpdating = false;
		}
	}

	const period = data.periodDetails;

	// Filter training notes to only include those within the period date range
	const periodNotes = $derived.by(() => {
		const startDate = dayjs(period.start);
		const endDate = period.end ? dayjs(period.end) : dayjs(); // If no end, use today

		return data.trainingNotes.filter((note) => {
			const noteDate = dayjs(note.date);
			return (
				(noteDate.isAfter(startDate, 'day') || noteDate.isSame(startDate, 'day')) &&
				(noteDate.isBefore(endDate, 'day') || noteDate.isSame(endDate, 'day'))
			);
		});
	});

	// Merge activities and notes, sorted by date (most recent first)
	type TimelineItem =
		| { type: 'activity'; data: (typeof period.activities)[0]; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	const timeline = $derived.by((): TimelineItem[] => {
		const items: TimelineItem[] = [
			...period.activities.map((activity) => ({
				type: 'activity' as const,
				data: activity,
				date: activity.start_time
			})),
			...periodNotes.map((note) => ({
				type: 'note' as const,
				data: note,
				date: note.date
			}))
		];

		return items.sort((a, b) => (a.date > b.date ? -1 : 1));
	});

	const saveNote = async (noteId: string, content: string, date: string) => {
		const success = await updateTrainingNote(noteId, content, date);
		if (success) {
			invalidate('app:training-notes');
		}
	};

	const handleDeleteNote = async (noteId: string) => {
		const success = await deleteTrainingNote(noteId);
		if (success) {
			invalidate('app:training-notes');
		}
	};

	const sportsByCategory = $derived.by(() => {
		const sports = period.sports;
		// Map category -> { category, icon, sports[], showAll }
		const categorySet: Set<SportCategory | 'Other'> = new Set(sports.categories);
		const map = new Map<
			string,
			{ category: SportCategory | 'Other'; icon: string; sports: string[]; showAll: boolean }
		>();

		// First, seed with explicit categories (these mean "all sports")
		for (const category of sports.categories) {
			map.set(category, {
				category: category,
				icon: getSportCategoryIcon(category),
				sports: [],
				showAll: true
			});
		}

		// Then, process individual sports
		for (const sport of sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				const key = category;
				// If category is already present in categories list, skip individual sport
				if (categorySet.has(key)) {
					continue;
				}
				// Otherwise, add sport to its category group
				if (!map.has(key)) {
					map.set(key, {
						category: key,
						icon: getSportCategoryIcon(category),
						sports: [],
						showAll: false
					});
				}
				map.get(key)!.sports.push(sport);
			} else {
				// Sports without category go to "Other"
				const other = 'Other';
				if (!categorySet.has(other)) {
					if (!map.has(other)) {
						map.set(other, {
							category: other,
							icon: getSportCategoryIcon(null),
							sports: [],
							showAll: false
						});
					}
					map.get(other)!.sports.push(sport);
				}
			}
		}

		return Array.from(map.values());
	});

	// Calculate summary statistics
	const summary = $derived.by(() => {
		const total = {
			count: period.activities.length,
			duration: 0,
			distance: 0,
			elevation: 0
		};

		for (const activity of period.activities) {
			total.duration += activity.duration ?? 0;
			total.distance += activity.distance ?? 0;
			total.elevation += activity.elevation ?? 0;
		}

		return total;
	});

	// Helper function to format distance
	const formatDistance = (meters: number): string => {
		if (meters === 0) return '0 km';
		const km = meters / 1000;
		return `${Math.round(km).toLocaleString('fr-fr')} km`;
	};

	// Helper function to format elevation
	const formatElevation = (meters: number): string => {
		if (meters === 0) return '0 m';
		return `${Math.round(meters).toLocaleString('fr-fr')} m`;
	};
</script>

<div class="mx-auto mt-4 flex flex-col gap-4">
	<div class="@container rounded-box rounded-t-none bg-base-100 p-4 shadow-md">
		<!-- Top row: Icon and Title/Date/Actions -->
		<div class="flex items-center gap-3">
			<!-- Icon -->
			<div class="text-2xl leading-none @lg:text-3xl">🗓️</div>

			<!-- Title and date -->
			<div class="flex-1">
				<div class="text-lg font-semibold @lg:text-xl">{period.name}</div>
				<div class="flex flex-wrap items-center gap-2 text-xs @lg:text-sm">
					<div class="opacity-70">
						{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
							? 'Ongoing'
							: dayjs(period.end).format('MMM D, YYYY')}
					</div>
					{#if sportsByCategory.length > 0}
						<div class="flex items-center gap-1.5">
							<span class="opacity-50">·</span>
							{#each sportsByCategory as group}
								<div
									class="tooltip tooltip-bottom text-base"
									data-tip={group.showAll
										? `${group.category} (all sub-sports)`
										: `${group.category}: ${group.sports.join(', ')}`}
								>
									{group.icon}
								</div>
							{/each}
						</div>
					{:else}
						<div class="opacity-50">· All sports</div>
					{/if}
					<!-- Action menu dropdown (always inline) -->
					<div class="dropdown dropdown-end">
						<div tabindex="0" role="button" class="btn btn-square opacity-100 btn-ghost btn-xs">
							⋮
						</div>
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
						<ul
							tabindex="0"
							class="dropdown-content menu z-[1] w-40 rounded-box bg-base-100 p-2 shadow"
						>
							<li>
								<button onclick={openEditModal}>
									<span>✏️</span>
									<span>Edit name</span>
								</button>
							</li>
							<li>
								<button onclick={() => (showDeleteModal = true)} class="text-error">
									<span>🗑️</span>
									<span>Delete</span>
								</button>
							</li>
						</ul>
					</div>
				</div>
			</div>
		</div>

		<!-- Period note section -->
		<div class="mt-4">
			{#if period.note}
				<div class="flex items-start gap-2">
					<div class="flex-1 rounded bg-base-200 p-3 text-sm whitespace-pre-wrap">
						{period.note}
					</div>
					<button
						class="btn btn-square btn-ghost btn-xs"
						onclick={openEditNoteModal}
						aria-label="Edit note"
					>
						✏️
					</button>
				</div>
			{:else}
				<button class="btn gap-2 btn-ghost btn-sm" onclick={openEditNoteModal}>
					<span>📝</span>
					<span>Add period description</span>
				</button>
			{/if}
		</div>
	</div>

	<!-- Activities section -->
	<div class="rounded-box bg-base-100 p-4 shadow-md">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="text-lg font-semibold">Activities & Notes</h2>
		</div>

		{#if period.activities.length > 0}
			<!-- Summary statistics -->
			<div class="mb-4 grid grid-cols-2 gap-3 rounded bg-base-200 p-4 md:grid-cols-4">
				<div class="flex flex-col">
					<div class="text-xs opacity-70">Total Activities</div>
					<div class="text-xl font-semibold">{summary.count}</div>
				</div>
				<div class="flex flex-col">
					<div class="text-xs opacity-70">Total Duration</div>
					<div class="text-xl font-semibold">
						{formatDurationHoursMinutes(summary.duration)}
					</div>
				</div>
				<div class="flex flex-col">
					<div class="text-xs opacity-70">Total Distance</div>
					<div class="text-xl font-semibold">{formatDistance(summary.distance)}</div>
				</div>
				<div class="flex flex-col">
					<div class="text-xs opacity-70">Total Elevation</div>
					<div class="text-xl font-semibold">{formatElevation(summary.elevation)}</div>
				</div>
			</div>

			<div class="flex flex-col gap-0">
				{#each timeline as item}
					{#if item.type === 'activity'}
						<ActivitiesListItem activity={item.data} showNote={true} />
					{:else}
						<TrainingNoteListItemCompact
							note={item.data}
							onEdit={(content, date) => saveNote(item.data.id, content, date)}
							onDelete={() => handleDeleteNote(item.data.id)}
						/>
					{/if}
				{/each}
			</div>
		{:else}
			<div class="py-8 text-center text-sm italic opacity-70">
				No activities in this training period yet
			</div>
		{/if}
	</div>
</div>

<!-- Edit name modal -->
{#if showEditModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Edit Training Period Name</h3>
			<div class="py-4">
				<label class="input">
					<span class="label">Name</span>
					<input
						type="text"
						bind:value={editedName}
						placeholder="Enter period name"
						class="w-full"
						disabled={isUpdating}
					/>
				</label>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={() => (showEditModal = false)} disabled={isUpdating}>
					Cancel
				</button>
				<button class="btn btn-primary" onclick={handleUpdate} disabled={isUpdating}>
					{#if isUpdating}
						<span class="loading loading-sm loading-spinner"></span>
						Updating...
					{:else}
						Update
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={() => (showEditModal = false)}>close</button>
		</form>
	</dialog>
{/if}

<!-- Delete confirmation modal -->
<DeleteModal
	bind:isOpen={showDeleteModal}
	title="Delete Training Period"
	description="Are you sure you want to delete this training period ? "
	itemPreview={period.name}
	onConfirm={handleDelete}
/>

<!-- Edit note modal -->
{#if showEditNoteModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">
				{period.note ? 'Edit' : 'Add'} training period description
			</h3>
			<div class="py-4">
				<label class="floating-label">
					<textarea
						bind:value={editedNote}
						placeholder="Add a description of this training period..."
						class="textarea w-full"
						rows="6"
						disabled={isUpdatingNote}
					></textarea>
					<span>Description</span>
				</label>
			</div>
			<div class="modal-action">
				<button class="btn" onclick={() => (showEditNoteModal = false)} disabled={isUpdatingNote}>
					Cancel
				</button>
				<button class="btn btn-primary" onclick={handleUpdateNote} disabled={isUpdatingNote}>
					{#if isUpdatingNote}
						<span class="loading loading-sm loading-spinner"></span>
						Saving...
					{:else}
						Save
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={() => (showEditNoteModal = false)}>close</button>
		</form>
	</dialog>
{/if}

<style>
	.rounded-box {
		border-radius: 8px;
	}
</style>
