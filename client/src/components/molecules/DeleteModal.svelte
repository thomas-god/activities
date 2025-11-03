<script lang="ts">
	interface Props {
		isOpen: boolean;
		title: string;
		description?: string;
		itemPreview?: string;
		onConfirm: () => Promise<void>;
	}

	let { isOpen = $bindable(false), title, description, itemPreview, onConfirm }: Props = $props();

	let isDeleting = $state(false);

	const handleCancel = () => {
		isOpen = false;
	};

	const handleConfirm = async () => {
		isDeleting = true;
		try {
			await onConfirm();
			isOpen = false;
		} finally {
			isDeleting = false;
		}
	};
</script>

{#if isOpen}
	<dialog class="modal-open modal">
		<div class="modal-box">
			<h3 class="text-lg font-bold">{title}</h3>
			<p class="py-4">
				{#if description}
					{description}
					<br />
				{/if}
				{#if itemPreview}
					<span class="mt-2 block text-sm italic opacity-70">
						{itemPreview}
					</span>
					<br />
				{/if}
				<strong>This action cannot be undone.</strong>
			</p>
			<div class="modal-action">
				<button class="btn" onclick={handleCancel} disabled={isDeleting}> Cancel </button>
				<button class="btn btn-error" onclick={handleConfirm} disabled={isDeleting}>
					{#if isDeleting}
						<span class="loading loading-sm loading-spinner"></span>
						Deleting...
					{:else}
						Delete
					{/if}
				</button>
			</div>
		</div>
		<form method="dialog" class="modal-backdrop">
			<button onclick={handleCancel}>close</button>
		</form>
	</dialog>
{/if}
