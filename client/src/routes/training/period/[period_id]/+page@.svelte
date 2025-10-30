<script lang="ts">
	import { dayjs, formatDurationHoursMinutes } from '$lib/duration';
	import {
		getSportCategory,
		sportCategoryIcons,
		getSportCategoryIcon,
		type SportCategory
	} from '$lib/sport';
	import { goto, invalidate } from '$app/navigation';
	import type { PageProps } from './$types';
	import type { TrainingPeriodDetails, TrainingNote } from './+page';
	import ActivitiesListItem from '../../../../organisms/ActivitiesListItem.svelte';
	import TrainingNoteListItem from '../../../../organisms/TrainingNoteListItem.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { updateTrainingNote, deleteTrainingNote } from '$lib/api/training';

	let { data }: PageProps = $props();

	let showDeleteModal = $state(false);
	let isDeleting = $state(false);
	let showEditModal = $state(false);
	let isUpdating = $state(false);
	let editedName = $state('');

	async function handleDelete() {
		isDeleting = true;
		try {
			const response = await fetch(`${PUBLIC_APP_URL}/api/training/period/${period.id}`, {
				method: 'DELETE',
				credentials: 'include',
				mode: 'cors'
			});

			if (response.ok) {
				await goto('/training/periods');
			} else {
				alert('Failed to delete training period');
			}
		} catch (error) {
			alert('Error deleting training period');
			console.error(error);
		} finally {
			isDeleting = false;
			showDeleteModal = false;
		}
	}

	function openEditModal() {
		editedName = period.name;
		showEditModal = true;
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
	<div class="rounded-box rounded-t-none bg-base-100 p-4 shadow-md">
		<div class="flex flex-row flex-wrap items-center gap-3">
			<div class="flex grow-1 flex-row items-center gap-3">
				<!-- Icon -->
				<div class="text-3xl leading-none">🗓️</div>

				<!-- Title and date -->
				<div>
					<div class="text-xl font-semibold">{period.name}</div>
					<div class="text-sm opacity-70">
						{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
							? 'Ongoing'
							: dayjs(period.end).format('MMM D, YYYY')}
					</div>
				</div>
			</div>

			<!-- Sports icons -->
			<div class="flex flex-row gap-5">
				<div class="flex flex-wrap items-center gap-2">
					{#each sportsByCategory as group}
						<div
							class="tooltip tooltip-bottom text-lg"
							data-tip={group.showAll
								? `${group.category} (all sub-sports)`
								: `${group.category}: ${group.sports.join(', ')}`}
						>
							{group.icon}
						</div>
					{:else}
						<div class="text-sm italic opacity-70">All sports</div>
					{/each}
				</div>

				<!-- Action buttons -->
				<div class="flex gap-2">
					<button
						class="btn btn-sm btn-primary"
						onclick={openEditModal}
						aria-label="Edit training period name"
					>
						✏️
					</button>
					<button
						class="btn btn-sm btn-error"
						onclick={() => (showDeleteModal = true)}
						aria-label="Delete training period"
					>
						🗑️
					</button>
				</div>
			</div>
		</div>

		{#if period.note}
			<div class="mt-4 max-w-xl rounded bg-base-200 p-3">{period.note}</div>
		{/if}
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

			<div class="flex flex-col gap-2">
				{#each timeline as item}
					{#if item.type === 'activity'}
						<ActivitiesListItem activity={item.data} />
					{:else}
						<div class="rounded-box border-l-4 border-warning/60 bg-warning/2">
							<div class="p-2 px-4">
								<TrainingNoteListItem
									note={item.data}
									onSave={(content, date) => saveNote(item.data.id, content, date)}
									onDelete={() => handleDeleteNote(item.data.id)}
								/>
							</div>
						</div>
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
{#if showDeleteModal}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">Delete Training Period</h3>
			<p class="py-4">
				Are you sure you want to delete "<strong>{period.name}</strong>"?
				<br />
				This action cannot be undone.
			</p>
			<div class="modal-action">
				<button class="btn" onclick={() => (showDeleteModal = false)} disabled={isDeleting}>
					Cancel
				</button>
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
			<button onclick={() => (showDeleteModal = false)}>close</button>
		</form>
	</dialog>
{/if}

<style>
	.rounded-box {
		border-radius: 8px;
	}
</style>
