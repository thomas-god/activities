<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import ActivityList from '../organisms/ActivityList.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let files: FileList | undefined = $state(undefined);
	let file_upload_content = $state('');

	const checkCanUpload = (fileList: FileList | undefined): fileList is FileList => {
		if (fileList) {
			return fileList.length > 0;
		}
		return false;
	};

	let can_upload = $derived.by(() => {
		return checkCanUpload(files);
	});

	const postActivity = async (fileList: FileList | undefined) => {
		if (!checkCanUpload(fileList)) {
			return;
		}

		let promises = [];
		for (let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i);
			promises.push(
				fetch(`${PUBLIC_APP_URL}/api/activity`, {
					body: file,
					method: 'POST'
				})
			);
		}

		let _res = await Promise.all(promises);
		file_upload_content = '';
		invalidate('app:activities');
	};

	let sorted_activities = $derived(
		data.activities.toSorted((a, b) => (a.start_time < b.start_time ? 1 : -1))
	);
</script>

<h1>Welcome to activities</h1>
<p>Upload a new activity</p>

<input
	type="file"
	class="file-input"
	accept=".fit"
	multiple
	bind:files
	bind:value={file_upload_content}
	id="activity_file"
	name="activity file"
/>
<button class="btn" disabled={!can_upload} onclick={() => postActivity(files)}>Upload</button>

<ActivityList activityList={sorted_activities} />
