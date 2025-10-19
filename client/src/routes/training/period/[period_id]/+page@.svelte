<script lang="ts">
	import { dayjs, formatDurationCompactWithUnits } from '$lib/duration';
	import {
		getSportCategory,
		sportCategoryIcons,
		getSportCategoryIcon,
		type SportCategory
	} from '$lib/sport';
	import { goto } from '$app/navigation';
	import type { PageProps } from './$types';
	import type { TrainingPeriodDetails } from './+page';
	import ActivitiesListItem from '../../../../organisms/ActivitiesListItem.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';

	let { data }: PageProps = $props();

	let showDeleteModal = $state(false);
	let isDeleting = $state(false);

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

	const period = data.periodDetails;

	const sportIcons = (sports: TrainingPeriodDetails['sports']): string[] => {
		const icons: Set<string> = new Set();

		for (const category of sports.categories) {
			if (category in sportCategoryIcons) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		for (const sport of sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		return Array.from(icons);
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
		<div class="flex items-center gap-4">
			<div class="text-3xl">🗓️</div>
			<div class="flex-1">
				<div class="text-xl font-semibold">{period.name}</div>
				<div class="text-sm opacity-70">
					{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
						? 'Ongoing'
						: dayjs(period.end).format('MMM D, YYYY')}
				</div>
			</div>
			<div class="flex items-center gap-2">
				{#each sportIcons(period.sports) as icon}
					<div class="text-lg">{icon}</div>
				{:else}
					<div class="italic opacity-70">All sports</div>
				{/each}
			</div>
			<button
				class="btn btn-sm btn-error"
				onclick={() => (showDeleteModal = true)}
				aria-label="Delete training period"
			>
				🗑️
			</button>
		</div>

		{#if period.note}
			<div class="mt-4 rounded bg-base-200 p-3">{period.note}</div>
		{/if}
	</div>

	<!-- Sports details section -->
	{#if period.sports.categories.length > 0 || period.sports.sports.length > 0}
		<div class="rounded-box bg-base-100 p-4 shadow-md">
			<details class="collapse-arrow collapse" open={false}>
				<summary class="collapse-title font-semibold">Sports</summary>
				<div class="collapse-content text-sm">
					{#each sportsByCategory as group}
						<div class="mb-4">
							<div class="mb-2 flex items-center gap-3">
								<div class="text-2xl">{group.icon}</div>
								<div class="font-semibold">{group.category}</div>
								{#if group.showAll}
									<span class="text-sm italic opacity-70">all sub-sports</span>
								{:else}
									<div class="text-sm italic opacity-70">
										{group.sports.length} sub-sports
									</div>
								{/if}
							</div>
							<div class="ml-11 flex flex-wrap gap-2">
								{#if !group.showAll}
									{#each group.sports as sport}
										<div class="badge badge-outline">{sport}</div>
									{/each}
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</details>
		</div>
	{/if}

	<!-- Activities section -->
	<div class="rounded-box bg-base-100 p-4 shadow-md">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="text-lg font-semibold">Activities</h2>
			<div class="badge badge-neutral">{period.activities.length}</div>
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
						{formatDurationCompactWithUnits(summary.duration)}
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
				{#each period.activities as activity}
					<ActivitiesListItem {activity} />
				{/each}
			</div>
		{:else}
			<div class="py-8 text-center text-sm italic opacity-70">
				No activities in this training period yet
			</div>
		{/if}
	</div>
</div>

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
