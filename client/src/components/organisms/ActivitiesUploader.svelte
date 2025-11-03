<script lang="ts">
	import { goto } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';

	let { activitiesUploadedCallback }: { activitiesUploadedCallback: () => void } = $props();

	let files: FileList | undefined = $state(undefined);
	let file_upload_content = $state('');
	let success = $state(false);

	const checkCanUpload = (files: FileList | undefined): files is FileList => {
		if (files) {
			return files.length > 0;
		}
		return false;
	};

	let can_upload = $derived.by(() => {
		return checkCanUpload(files);
	});

	let pending = $state(false);
	const postActivity = async (fileList: FileList | undefined) => {
		if (!checkCanUpload(fileList)) {
			return;
		}

		const formData = new FormData();

		for (let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i)!;
			formData.append(file.name, file);
		}

		pending = true;
		let res = await fetch(`${PUBLIC_APP_URL}/api/activity`, {
			body: formData,
			method: 'POST',
			mode: 'cors',
			credentials: 'include'
		});
		pending = false;

		if (res.status === 401) {
			goto('/login');
		}

		success = true;
		file_upload_content = '';
		activitiesUploadedCallback();
	};
</script>

<fieldset class="fieldset rounded-box border-base-300 bg-base-100 p-2">
	<legend class="fieldset-legend text-base">Upload new activities</legend>
	<div class="join gap-3">
		<input
			type="file"
			class="file-input"
			accept=".fit,.fit.gz,.tcx,.tcx.gz"
			multiple
			bind:files
			bind:value={file_upload_content}
			id="activity_file"
			name="activity file"
		/>
		<button
			class="btn rounded-lg btn-primary"
			disabled={!can_upload}
			onclick={() => postActivity(files)}
		>
			{#if pending}
				<span class="loading loading-spinner"></span>
			{:else}
				Upload
			{/if}
		</button>
	</div>
	<p class="label">.fit and .tcx files are supported, max 1 GB</p>
	{#if success}
		<div class="mt-2 rounded-box bg-success/20 p-3 text-success-content">
			Files successfully uploaded !
		</div>
	{/if}
</fieldset>
