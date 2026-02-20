<script lang="ts">
	import EditButton from '$components/atoms/EditButton.svelte';
	import SaveButton from '$components/atoms/SaveButton.svelte';
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
			<div class="flex flex-col gap-2">
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
			<SaveButton callback={handleSave} text="Save" />
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-2">
		<div class="flex flex-col gap-2">
			<div class="flex flex-row items-center text-sm font-medium">
				<span class="pr-0.5">Nutrition</span>
				{#if nutrition !== null}
					<EditButton callback={() => (editMode = true)} />
				{/if}
			</div>
			{#if nutrition === null}
				<button
					class="mr-auto link text-sm link-hover opacity-70"
					onclick={() => (editMode = true)}
				>
					Add nutrition
				</button>
			{:else}
				<div class="flex items-center gap-2">
					<span class="badge">
						{getBonkStatusIcon(nutrition.bonk_status)}
						{getBonkStatusLabel(nutrition.bonk_status)}
					</span>
				</div>
			{/if}
		</div>
		{#if nutrition?.details}
			<div class="rounded-lg bg-base-200 p-3 text-sm">
				<p class="whitespace-pre-wrap">{nutrition.details}</p>
			</div>
		{/if}
	</div>
{/if}
