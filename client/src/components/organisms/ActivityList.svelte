<script lang="ts">
	import type { Activity, ActivityListSummaryItems, TrainingNote } from '$lib/api';
	import ActivityListItem from '$components/organisms/ActivityListItem.svelte';
	import TrainingNoteListItemCompact from '$components/organisms/TrainingNoteListItemCompact.svelte';
	import { toTitleCase } from '$lib/utils';
	import ActivityListItemWithDetails from './ActivityListItemWithDetails.svelte';

	let {
		selectedActivityId,
		selectActivityCallback,
		activityListFormat,
		timeline,
		showGroupNumberOfActivities = true
	}: {
		timeline: Map<string, TimelineItem[]>;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		activityListFormat: ActivityListSummaryItems;
		showGroupNumberOfActivities?: boolean;
	} = $props();

	// Merge activities and notes, sorted by date (most recent first)
	export type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	let summaryTemplate = $derived(
		activityListFormat
			.map((item) => {
				if (item.type === 'workoutType') {
					return '120px';
				} else if (item.type === 'rpe') {
					return '80px';
				}

				return `minmax(70px, min-content)`;
			})
			.join(' ')
	);
	let summaryHeaders = $derived(
		activityListFormat.map((item) => {
			if (item.type === 'rpe') {
				return 'RPE';
			} else if (item.type === 'workoutType') {
				return 'Workout';
			} else {
				return toTitleCase(item.value);
			}
		})
	);
	let containerWidth = $state(0);
</script>

<div class="@container" bind:clientWidth={containerWidth}>
	{#if timeline.size > 0}
		{#if containerWidth > 650}
			<!-- Larger screen widths -->
			<div
				class="grid-container"
				style:grid-template-columns={`minmax(auto, 60px) minmax(175px, 1fr) ${summaryTemplate}`}
			>
				<!-- Items -->
				{#each timeline as [group, items], idx}
					<div
						class="sticky-left bg-base-100 py-2 text-xs font-semibold tracking-wide text-base-content/60 uppercase"
						style:grid-column={'1 / span 2'}
					>
						{group}
						{#if showGroupNumberOfActivities}
							&nbsp - {items.filter((item) => item.type === 'activity').length} activities
						{/if}
					</div>
					<!-- Metrics headers aligned to first group of the timeline -->
					{#if idx === 0}
						{#each summaryHeaders as header, header_index}
							<div style:grid-column-start={header_index + 1 + 2} class="text-xs text-center py-2">
								{header}
							</div>
						{/each}
					{/if}
					{#each items as item (item.date)}
						{#if item.type === 'activity'}
							<ActivityListItemWithDetails
								activity={item.data}
								onClick={() => {
									if (selectActivityCallback !== undefined) {
										selectActivityCallback(item.data.id);
									}
								}}
								isSelected={selectedActivityId === item.data.id}
								listFormat={activityListFormat}
							/>
						{:else}
							<TrainingNoteListItemCompact note={item.data} />
						{/if}
					{/each}
				{/each}
			</div>
		{:else}
			<!-- Smaller screen widths -->
			<div class="small-container">
				{#each timeline as [group, items], idx}
					<div
						class="bg-base-100 py-2 text-xs font-semibold tracking-wide text-base-content/60 uppercase"
					>
						{group}
						{#if showGroupNumberOfActivities}
							&nbsp - {items.filter((item) => item.type === 'activity').length} activities
						{/if}
					</div>

					{#each items as item (item.date)}
						{#if item.type === 'activity'}
							<div class="flex flex-row">
								<ActivityListItem
									activity={item.data}
									onClick={() => {
										if (selectActivityCallback !== undefined) {
											selectActivityCallback(item.data.id);
										}
									}}
									isSelected={selectedActivityId === item.data.id}
								/>
							</div>
						{:else}
							<TrainingNoteListItemCompact note={item.data} />
						{/if}
					{/each}
				{/each}
			</div>
		{/if}
	{:else}
		<div class="py-8 text-center text-sm italic opacity-70">No activities or notes found</div>
	{/if}
</div>

<style>
	.small-container {
		display: flex;
		flex-direction: column;
		@container (width >= 650px) {
			display: none;
		}
	}

	.grid-container {
		display: grid;
		width: 100%;
		overflow-x: scroll;

		@container (width < 650px) {
			display: none;
		}
	}

	.sticky-left {
		position: sticky;
		left: 0;
	}
</style>
