<script lang="ts">
	import { PUBLIC_APP_URL } from '$env/static/public';

	let files: FileList | undefined = $state(undefined);
	let file_upload_content: string = $state('');

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

		const file = fileList.item(0)!;

		let res = await fetch(`${PUBLIC_APP_URL}/activity`, {
			body: file,
			method: 'POST'
		});

		console.log(res.status);
	};
</script>

<h1>Welcome to activities</h1>
<p>Upload a new activity</p>

<input
	type="file"
	class="file-input"
	accept=".fit"
	bind:files
	bind:value={file_upload_content}
	id="activity_file"
	name="activity file"
/>
<button class="btn" disabled={!can_upload} onclick={() => postActivity(files)}>Upload</button>
