<script lang="ts">
	import type { ActivityListItem } from '$lib/types/activity';

	let { activity }: { activity: ActivityListItem } = $props();

	let duration = $derived.by(() => {
		if (activity.duration === null) {
			return null;
		}

		const hours = Math.floor(activity.duration / 3600);
		const minutes = Math.floor((activity.duration - hours * 3600) / 60);
		const seconds = activity.duration - hours * 3600 - minutes * 60;

		return { hours, minutes, seconds };
	});
</script>

<div>
	{activity.sport}:

	{#if duration !== null}
		{`${String(duration.hours).padStart(2, '0')}:${String(duration.minutes).padStart(2, '0')}:${String(duration.seconds).padStart(2, '0')}`}
	{/if}

	{#if activity.calories !== null}
		({activity.calories} kcal)
	{/if}
</div>
