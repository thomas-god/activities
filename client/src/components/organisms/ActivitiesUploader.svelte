<script lang="ts">
	import { goto } from '$app/navigation';
	import { postActivities } from '$lib/api';

	let { activitiesUploadedCallback }: { activitiesUploadedCallback: () => void } = $props();

	let files: FileList | undefined = $state(undefined);
	let file_upload_content = $state('');
	let formState: 'NotSent' | 'Pending' | 'Success' | 'Error' = $state('NotSent');
	let duplicatedFiles: string[] = $state([]);
	let invalidFiles: string[] = $state([]);

	const checkCanUpload = (files: FileList | undefined): files is FileList => {
		if (files) {
			return files.length > 0;
		}
		return false;
	};

	let can_upload = $derived.by(() => {
		return checkCanUpload(files);
	});

	const postActivitiesCallback = async () => {
		if (!checkCanUpload(files)) {
			return;
		}

		const formData = new FormData();

		for (let i = 0; i < files.length; i++) {
			const file = files.item(i)!;
			formData.append(file.name, file);
		}

		formState = 'Pending';
		let res = await postActivities(formData);

		if (res.type === 'error') {
			formState = 'Error';
		}

		if (res.type === 'success') {
			formState = 'Success';
			for (const { file, reason } of res.unprocessed) {
				if (reason === 'duplicated') {
					duplicatedFiles.push(file);
				} else {
					invalidFiles.push(file);
				}
			}
		}

		file_upload_content = '';

		if (res.type === 'authentication-error') {
			goto('/login');
		}

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
			onclick={() => postActivitiesCallback()}
		>
			{#if formState === 'Pending'}
				<span class="loading loading-spinner"></span>
			{:else}
				Upload
			{/if}
		</button>
	</div>
	<p class="label">.fit and .tcx files are supported, max 1 GB</p>
	{#if formState === 'Success'}
		<div class="mt-2 rounded-box bg-success/20 p-3 text-success-content">
			Files successfully uploaded !
		</div>
		{#if duplicatedFiles.length > 0}
			<div class="mt-2 rounded-box bg-warning/20 p-3 text-warning-content">
				Some files were already imported and have been skipped
				<ul>
					{#each duplicatedFiles as file}
						<li>
							{file}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if invalidFiles.length > 0}
			<div class="mt-2 rounded-box bg-error/20 p-3 text-error-content">
				Some files could not be processed
				<ul>
					{#each invalidFiles as file}
						<li>
							{file}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	{:else if formState === 'Error'}
		<div class="mt-2 rounded-box bg-error/20 p-3 text-error-content">
			An error occurred when trying to upload activities files, try again later.
		</div>
	{/if}
</fieldset>
