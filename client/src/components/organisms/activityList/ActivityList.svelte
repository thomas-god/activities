<script lang="ts">
	import type { Activity, ActivityListSummaryItems, TrainingNote } from '$lib/api';
	import { toTitleCase } from '$lib/utils';
	import TrainingNoteComponent from './TrainingNote.svelte';
	import ActivityComponent from './Activity.svelte';

	let {
		selectedActivityId,
		selectActivityCallback,
		activityListFormat,
		timeline,
		noteChangedCallback,
		showGroupNumberOfActivities = true
	}: {
		timeline: Map<string, TimelineItem[]>;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		noteChangedCallback: () => void;
		activityListFormat: ActivityListSummaryItems;
		showGroupNumberOfActivities?: boolean;
	} = $props();

	// Merge activities and notes, sorted by date (most recent first)
	export type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	let containerWidth = $state(0);
	let metricFormats = $derived.by(() => {
		const formats = [];
		let remainingWidth = containerWidth;
		for (const item of activityListFormat) {
			let width = 75;
			if (item.type === 'workoutType') {
				width = 120;
			} else if (item.type === 'rpe') {
				width = 80;
			}

			formats.push({ format: item, width, show: remainingWidth >= width });
			remainingWidth -= width;
		}
		return formats;
	});
	let headersTotalWidth = $derived(metricFormats.reduce((acc, cur) => acc + cur.width, 0));

	const headerTitle = (item: ActivityListSummaryItems[number]): string => {
		if (item.type === 'rpe') {
			return 'RPE';
		} else if (item.type === 'workoutType') {
			return 'Workout';
		} else {
			return toTitleCase(item.value);
		}
	};
</script>

<div class="@container">
	<!-- Larger screen widths -->
	<div class="flex flex-col gap-1" bind:clientWidth={containerWidth}>
		<!-- Items -->
		{#each timeline as [group, items], idx}
			<div class="flex flex-row justify-between overflow-hidden">
				<div
					class="bg-base-100 py-2 shrink-0 text-xs font-semibold tracking-wide text-base-content/60 uppercase"
				>
					{group}
					{#if showGroupNumberOfActivities}
						&nbsp - {items.filter((item) => item.type === 'activity').length} activities
					{/if}
				</div>
				<!-- Metrics headers aligned to first group of the timeline -->
				<!-- Activity.svelte: 350px min width + 4px border + 4px gap -->
				{#if idx === 0 && headersTotalWidth + 358 <= containerWidth}
					<div class="flex flex-row text-center">
						{#each metricFormats as header, header_index}
							<div
								class="text-xs text-center py-2"
								style:width={`${header.width}px`}
								hidden={!header.show}
							>
								{headerTitle(header.format)}
							</div>
						{/each}
					</div>
				{/if}
			</div>
			{#each items as item (item.date)}
				{#if item.type === 'activity'}
					<ActivityComponent
						activity={item.data}
						onClick={() => selectActivityCallback(item.data.id)}
						isSelected={selectedActivityId === item.data.id}
						listFormat={metricFormats}
					/>
				{:else}
					<div class="training-note">
						<TrainingNoteComponent note={item.data} noteChanged={noteChangedCallback} />
					</div>
				{/if}
			{/each}
		{:else}
			<div class="py-8 text-center text-sm italic opacity-70">No activities or notes found</div>
		{/each}
	</div>
</div>

<style>
	.training-note {
		max-width: min(75vw, 500px);
	}
</style>
