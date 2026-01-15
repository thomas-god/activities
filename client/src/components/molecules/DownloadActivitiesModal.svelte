<script lang="ts">
	import { downloadAllActivities } from '$lib/api';

	interface Props {
		isOpen: boolean;
		activityCount: number;
	}

	let { isOpen = $bindable(false), activityCount }: Props = $props();

	let isDownloading = $state(false);

	const handleCancel = () => {
		if (!isDownloading) {
			isOpen = false;
		}
	};

	const handleConfirm = async () => {
		isDownloading = true;
		try {
			await downloadAllActivities();
			isOpen = false;
		} catch (error) {
			console.error('Failed to download activities:', error);
			alert('Failed to download activities. Please try again.');
		} finally {
			isDownloading = false;
		}
	};
</script>

{#if isOpen}
	<dialog class="modal-open modal">
		<div class="modal-box">
			{#if isDownloading}
				<!-- Downloading state -->
				<h3 class="text-lg font-bold">Preparing download...</h3>
				<div class="flex flex-col items-center gap-4 py-8">
					<span class="loading loading-lg loading-spinner"></span>
					<p class="text-center text-sm">
						Your activity history is being prepared. This may take a few minutes depending on the
						number of activities.
					</p>
					<p class="text-center text-sm text-base-content/60">Please don't close this window.</p>
				</div>
			{:else}
				<!-- Confirmation state -->
				<h3 class="text-lg font-bold">Download activity history</h3>
				<p class="py-4">
					This will download all your activities ({activityCount}
					{activityCount === 1 ? 'activity' : 'activities'}) as a ZIP file containing the original
					files (.fit, .tcx).
				</p>
				<p class="pb-4 text-sm text-base-content/60">
					The download may take a few minutes to prepare, do not close this window.
				</p>
				<div class="modal-action">
					<button class="btn btn-ghost" onclick={handleCancel}>Cancel</button>
					<button class="btn btn-primary" onclick={handleConfirm}>
						<span class="text-lg">⬇️</span>
						Download
					</button>
				</div>
			{/if}
		</div>
		{#if !isDownloading}
			<form method="dialog" class="modal-backdrop">
				<button onclick={handleCancel}>close</button>
			</form>
		{/if}
	</dialog>
{/if}
