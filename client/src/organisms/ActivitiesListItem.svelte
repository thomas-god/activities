<script lang="ts">
	import { formatDuration, formatRelativeDuration, dayjs } from '$lib/duration';
	import { getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import type { ActivityListItem } from '../routes/+page';

	let { activity }: { activity: ActivityListItem } = $props();

	let title = $derived(
		activity.name === null || activity.name === '' ? activity.sport : activity.name
	);

	const categoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};
</script>

<a
	href={`/activity/${activity.id}`}
	class={`item flex flex-1 items-center p-3 ${categoryClass(activity.sport_category)}`}
>
	<div class={`icon ${categoryClass(activity.sport_category)}`}>
		{getSportCategoryIcon(activity.sport_category)}
	</div>
	<div class="flex-1">
		<div class="flex flex-col">
			<div class="mb-1 font-semibold">{title}</div>
			<div class="text-xs font-light">
				{formatRelativeDuration(dayjs(activity.start_time), dayjs())} . {dayjs(
					activity.start_time
				).format('MMM D, YYYY')}
			</div>
		</div>
	</div>
	<div class="font-semibold sm:text-lg">
		{formatDuration(activity.duration)}
	</div>
</a>

<style>
	.item:hover {
		background: #f7fafc;
	}

	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.item.cycling {
		border-left-color: var(--color-cycling);
	}

	.item.running {
		border-left-color: var(--color-running);
	}

	.item.other {
		border-left-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
	}

	.icon.cycling {
		background: var(--color-cycling-background);
		color: var(--color-cycling);
	}

	.icon.running {
		background: var(--color-running-background);
		color: var(--color-running);
	}

	.icon.other {
		background: var(--color-other-background);
		color: var(--color-other);
	}
</style>
