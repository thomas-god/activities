<script lang="ts">
	import {
		BONK_STATUS_VALUES,
		getBonkStatusLabel,
		getBonkStatusColor,
		getBonkStatusIcon,
		type BonkStatus,
		type Nutrition
	} from '$lib/nutrition';

	let {
		nutrition: initialNutrition,
		editCallback
	}: {
		nutrition: Nutrition | null;
		editCallback: (newNutrition: Nutrition | null) => Promise<void>;
	} = $props();

	let nutrition = $state(initialNutrition);
	let editMode = $state(false);

	const handleSave = () => {
		editMode = false;
		editCallback(nutrition);
	};

	const handleCancel = () => {
		editMode = false;
		nutrition = initialNutrition;
	};

	const updateNutritionBonkStatus = (newStatus: BonkStatus) => {
		if (nutrition === null) {
			nutrition = {
				bonk_status: newStatus,
				details: null
			};
		} else {
			nutrition.bonk_status = newStatus;
		}
	};
</script>

{#if editMode}
	<div class="flex flex-col gap-2">
		<div class="text-sm font-medium">Nutrition & Hydration</div>
		<div class="flex flex-wrap gap-2">
			<button
				class={`btn btn-sm ${nutrition === null ? 'btn-active' : 'btn-ghost'}`}
				onclick={() => (nutrition = null)}
			>
				Clear
			</button>
			{#each BONK_STATUS_VALUES as status}
				<button
					class={`btn btn-sm ${nutrition !== null && nutrition.bonk_status === status ? `btn-active ${getBonkStatusColor(status)}` : 'btn-ghost'}`}
					onclick={() => updateNutritionBonkStatus(status)}
				>
					{getBonkStatusIcon(status)}
					{getBonkStatusLabel(status)}
				</button>
			{/each}
		</div>
		{#if nutrition !== null && nutrition.bonk_status !== null}
			<div class="form-control">
				<label class="label" for="nutrition-details">
					<span class="label-text text-xs">Details (optional)</span>
				</label>
				<textarea
					id="nutrition-details"
					class="textarea-bordered textarea textarea-sm"
					placeholder="e.g., Forgot to eat breakfast, only had water..."
					bind:value={nutrition.details}
					rows="2"
				></textarea>
			</div>
		{/if}
		<div class="flex gap-2">
			<button class="btn btn-sm btn-primary" onclick={handleSave}>💾 Save</button>
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
		</div>
	</div>
{:else}
	<div class="flex items-center gap-2">
		<div class="text-sm font-medium">Nutrition:</div>
		{#if nutrition === null}
			<span class="badge">Not set</span>
		{:else}
			<span class="badge">
				{getBonkStatusIcon(nutrition.bonk_status)}
				{getBonkStatusLabel(nutrition.bonk_status)}
			</span>
			{#if nutrition.details}
				<span class="text-xs text-base-content/70 italic">"{nutrition.details}"</span>
			{/if}
		{/if}
		<button class="btn btn-ghost btn-xs" onclick={() => (editMode = true)}>✏️ Edit</button>
	</div>
{/if}
