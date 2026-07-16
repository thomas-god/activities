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
		renderByChunk = false,
		showGroupNumberOfActivities = true
	}: {
		timeline: Map<string, TimelineItem[]>;
		selectedActivityId: string | null;
		selectActivityCallback: (id: string) => void;
		noteChangedCallback: () => void;
		activityListFormat: ActivityListSummaryItems;
		renderByChunk?: boolean;
		showGroupNumberOfActivities?: boolean;
	} = $props();

	// Merge activities and notes, sorted by date (most recent first)
	export type TimelineItem =
		| { type: 'activity'; data: Activity; date: string }
		| { type: 'note'; data: TrainingNote; date: string };

	const CHUNK_SIZE = 50;
	let renderedCount = $state(CHUNK_SIZE);
	let scrollElement: HTMLDivElement | null = $state(null);
	let flattenTimeline = $derived.by(() => {
		const flattenTimeline = [];
		for (const [group, items] of timeline) {
			flattenTimeline.push({ type: 'header' as const, group });
			for (const item of items) {
				flattenTimeline.push({ type: 'row' as const, item });
			}
		}
		return flattenTimeline;
	});
	let visibleItems = $derived(
		renderByChunk ? flattenTimeline.slice(0, renderedCount) : flattenTimeline
	);
	let hasMore = $derived(renderedCount < flattenTimeline.length);
	const loadMore = () => {
		renderedCount = Math.min(renderedCount + CHUNK_SIZE, flattenTimeline.length);
	};
	// action: observes the sentinel node, triggers loadMore when it scrolls into view
	const sentinelObserver = (node: HTMLElement) => {
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting && hasMore) {
					loadMore();
				}
			},
			{ root: scrollElement, rootMargin: '200px' } // start loading before it's fully visible
		);
		observer.observe(node);
		return {
			destroy() {
				observer.disconnect();
			}
		};
	};

	let containerHeight = $derived(renderByChunk ? 'h-[80vh]' : 'h-auto');
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
	// Activity.svelte: 350px min width + 4px border + 4px gap
	let headersOverflow = $derived(headersTotalWidth + 358 > containerWidth);

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

<div
	class={`@container flex flex-col gap-1 overflow-y-scroll ${containerHeight}`}
	bind:clientWidth={containerWidth}
	bind:this={scrollElement}
>
	{#each visibleItems as item, idx}
		{#if item.type === 'header'}
			<div class="flex flex-row justify-between overflow-x-hidden shrink-0">
				<div
					class="bg-base-100 py-2 text-xs font-semibold tracking-wide text-base-content/60 uppercase"
				>
					{item.group}
					{#if showGroupNumberOfActivities}
						&nbsp - {timeline.get(item.group)!.filter((item) => item.type === 'activity').length} activities
					{/if}
				</div>
				<!-- Metrics headers aligned to first group of the timeline -->
				{#if idx === 0 && !headersOverflow}
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
		{:else if item.type === 'row' && item.item.type === 'activity'}
			<div>
				<ActivityComponent
					activity={item.item.data}
					onClick={() => selectActivityCallback(item.item.data.id)}
					isSelected={selectedActivityId === item.item.data.id}
					listFormat={metricFormats}
				/>
			</div>
		{:else if item.type === 'row' && item.item.type === 'note'}
			<div class="training-note">
				<TrainingNoteComponent note={item.item.data} noteChanged={noteChangedCallback} />
			</div>
		{/if}
	{:else}
		<div class="py-8 text-center text-sm italic opacity-70">No activities or notes found</div>
	{/each}

	{#if hasMore}
		<div use:sentinelObserver class="sentinel"></div>
	{/if}
</div>

<style>
	.training-note {
		max-width: min(75vw, 500px);
	}
</style>
