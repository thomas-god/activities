<script lang="ts">
	import { formatDuration } from '$lib/duration';
	import type { ActivityListItem } from '../routes/+page';

	let { activity }: { activity: ActivityListItem } = $props();
</script>

<div>
	{activity.sport}:
</div>

<div>
	{`${formatDuration(activity.duration)}`}
</div>

<div>
	{activity.start_time}
</div>
