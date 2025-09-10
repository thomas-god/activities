<script lang="ts">
	import { invalidate } from '$app/navigation';
	import ActivityList from '../organisms/ActivityList.svelte';
	import ActivitiesUploader from '../organisms/ActivitiesUploader.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const activitiesUploadedCallback = () => {
		invalidate('app:activities');
	};

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);
</script>

<div class="w-sm mx-auto mb-2">
	<ActivitiesUploader {activitiesUploadedCallback} />
</div>

<ActivityList activityList={sorted_activities} />
