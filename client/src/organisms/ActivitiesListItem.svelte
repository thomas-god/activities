<script lang="ts">
	import { formatDuration, formatRelativeDuration, dayjs } from '$lib/duration';
	import type { ActivityListItem } from '../routes/+page';

	let { activity }: { activity: ActivityListItem } = $props();

	let title = $derived(
		activity.name === null || activity.name === '' ? activity.sport : activity.name
	);
	let sport = $derived(activity.sport.split(' ').join('').toLowerCase());

	const icons = {
		running: '🏃',
		cycling: '🚴',
		strengthtraining: '💪'
	};
</script>

<a href={`/activity/${activity.id}`} class={`item flex flex-1 items-center p-3 ${sport}`}>
	<div class={`icon ${sport}`}>{icons[sport]}</div>
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
		border-left-color: #4299e1;
	}

	.item.running {
		border-left-color: #ed8936;
	}

	.item.strengthtraining {
		border-left-color: #48bb78;
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
		background: #ebf8ff;
		color: #4299e1;
	}

	.icon.running {
		background: #feebc8;
		color: #ed8936;
	}

	.icon.strengthtraining {
		background: #c6f6d5;
		color: #48bb78;
	}
</style>
