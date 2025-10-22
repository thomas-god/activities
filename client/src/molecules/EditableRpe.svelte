<script lang="ts">
	import { RPE_VALUES, getRpeLabelAsScale, getRpeColor, getRpeButtonColor } from '$lib/rpe';

	let {
		rpe: initialRpe,
		editCallback
	}: { rpe: number | null; editCallback: (newRpe: number | null) => Promise<void> } = $props();

	let rpe = $state(initialRpe);
	let editMode = $state(false);

	const handleSave = () => {
		editMode = false;
		editCallback(rpe);
	};

	const handleCancel = () => {
		editMode = false;
		rpe = initialRpe;
	};
</script>

{#if editMode}
	<div class="flex flex-col gap-2">
		<div class="text-sm font-medium">RPE (Rate of Perceived Exertion)</div>
		<div class="flex flex-wrap gap-2">
			<button
				class={`btn btn-sm ${rpe === null ? 'btn-active' : 'btn-ghost'}`}
				onclick={() => (rpe = null)}
			>
				Clear
			</button>
			{#each RPE_VALUES as value}
				<button
					class={`btn btn-sm ${rpe === value ? `${getRpeButtonColor(value)} border-2 border-base-content` : getRpeButtonColor(value)}`}
					onclick={() => (rpe = value)}
				>
					{value}
				</button>
			{/each}
		</div>
		<div class="flex gap-2">
			<button class="btn btn-sm btn-primary" onclick={handleSave}>💾 Save</button>
			<button class="btn btn-ghost btn-sm" onclick={handleCancel}>Cancel</button>
		</div>
	</div>
{:else}
	<div class="flex items-center gap-2">
		<div class="text-sm font-medium">RPE:</div>
		<span class={`badge ${rpe === null ? '' : getRpeColor(rpe)}`}>{getRpeLabelAsScale(rpe)}</span>
		<button class="btn opacity-75 btn-ghost btn-sm" onclick={() => (editMode = true)}> ✏️ </button>
	</div>
{/if}

<style>
	.rpe-easy {
		background-color: var(--color-rpe-easy);
		color: var(--color-rpe-easy-text);
	}

	.rpe-moderate {
		background-color: var(--color-rpe-moderate);
		color: var(--color-rpe-moderate-text);
	}

	.rpe-hard {
		background-color: var(--color-rpe-hard);
		color: var(--color-rpe-hard-text);
	}

	.rpe-very-hard {
		background-color: var(--color-rpe-very-hard);
		color: var(--color-rpe-very-hard-text);
	}

	.rpe-max {
		background-color: var(--color-rpe-max);
		color: var(--color-rpe-max-text);
	}

	/* Ensure good contrast for badge */
	.badge.rpe-easy,
	.badge.rpe-moderate,
	.badge.rpe-hard,
	.badge.rpe-very-hard,
	.badge.rpe-max {
		border: 1px solid currentColor;
	}
</style>
