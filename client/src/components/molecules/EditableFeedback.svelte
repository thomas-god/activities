<script lang="ts">
	let {
		feedback: initialFeedback,
		editCallback
	}: {
		feedback: string | null;
		editCallback: (newFeedback: string | null) => Promise<void>;
	} = $props();

	let feedback = $state(initialFeedback);
	let editMode = $state(false);

	const handleSave = () => {
		editMode = false;
		editCallback(feedback);
	};

	const handleCancel = () => {
		editMode = false;
		feedback = initialFeedback;
	};
</script>

{#if editMode}
	<div class="flex flex-col gap-2">
		<div class="text-sm font-medium">Note</div>
		<div class="flex flex-col gap-2">
			<label class="label" for="activity-feedback">
				<span class="label-text text-xs">Share your thoughts about this activity</span>
			</label>
			<textarea
				id="activity-feedback"
				class="textarea-bordered textarea"
				placeholder="e.g., Great run! Felt strong throughout. Weather was perfect and pacing was on point..."
				bind:value={feedback}
				rows="4"
			></textarea>
		</div>
		<div class="flex gap-2">
			<button class="btn btn-sm btn-primary" onclick={handleSave}>💾 Save</button>
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
			{#if feedback}
				<button
					class="btn btn-ghost btn-sm"
					onclick={() => {
						feedback = null;
					}}
				>
					Clear
				</button>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-2">
		<div class="flex flex-row text-sm font-medium">
			<span>Note</span>
			{#if feedback !== null && feedback !== ''}
				<button class="btn ml-auto btn-ghost btn-xs" onclick={() => (editMode = true)}>
					✏️ Edit
				</button>
			{/if}
		</div>
		{#if feedback === null || feedback === ''}
			<button class="mr-auto link text-sm link-hover opacity-70" onclick={() => (editMode = true)}>
				Add note
			</button>
		{:else}
			<div class="rounded-lg bg-base-200 p-3 text-sm">
				<p class="whitespace-pre-wrap">{feedback}</p>
			</div>
		{/if}
	</div>
{/if}
