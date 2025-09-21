<script lang="ts">
	import { goto } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';

	let { activitiesUploadedCallback }: { activitiesUploadedCallback: () => void } = $props();

	let files: FileList | undefined = $state(undefined);
	let file_upload_content = $state('');

	const checkCanUpload = (files: FileList | undefined): files is FileList => {
		if (files) {
			return files.length > 0;
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

		const formData = new FormData();

		for (let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i)!;
			formData.append(file.name, file);
		}

		let res = await fetch(`${PUBLIC_APP_URL}/api/activity`, {
			body: formData,
			method: 'POST',
			mode: 'cors',
			credentials: 'include'
		});

		if (res.status === 401) {
			goto('/login');
		}

		file_upload_content = '';
		activitiesUploadedCallback();
	};
</script>

<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
	<legend class="fieldset-legend">Upload new activities</legend>
	<div class="join gap-3">
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
		<button
			class="btn bg-accent text-accent-content disabled:bg-base-200/10 disabled:text-base-content/20"
			disabled={!can_upload}
			onclick={() => postActivity(files)}>Upload</button
		>
	</div>
	<p class="label">.fit files are supported</p>
</fieldset>
