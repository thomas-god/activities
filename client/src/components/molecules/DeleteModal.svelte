<script lang="ts">
	interface Props {
		isOpen: boolean;
		title: string;
		description?: string;
		itemPreview?: string;
		warning?: string;
		onConfirm: () => Promise<void>;
	}

	let {
		isOpen = $bindable(false),
		title,
		description,
		itemPreview,
		warning,
		onConfirm
	}: Props = $props();

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
			{#if warning}
				<div role="alert" class="mb-4 alert alert-warning">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-6 w-6 shrink-0 stroke-current"
						fill="none"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
						/>
					</svg>
					<span>{warning}</span>
				</div>
			{/if}
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
