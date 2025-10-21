<script lang="ts">
	let {
		rpe: initialRpe,
		editCallback
	}: { rpe: number | null; editCallback: (newRpe: number | null) => Promise<void> } = $props();

	let rpe = $state(initialRpe);
	let editMode = $state(false);

	const rpeValues = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

	const getRpeLabel = (value: number | null): string => {
		if (value === null) return 'Not set';
		return `${value}/10`;
	};

	const getRpeColor = (value: number | null): string => {
		if (value === null) return 'badge-ghost';
		if (value <= 2) return 'rpe-light-blue';
		if (value <= 4) return 'rpe-light-green';
		if (value <= 7) return 'rpe-light-yellow';
		if (value <= 9) return 'rpe-orange';
		return 'rpe-red';
	};

	const getButtonColor = (value: number): string => {
		if (value <= 2) return 'rpe-light-blue';
		if (value <= 4) return 'rpe-light-green';
		if (value <= 7) return 'rpe-light-yellow';
		if (value <= 9) return 'rpe-orange';
		return 'rpe-red';
	};

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
			{#each rpeValues as value}
				<button
					class={`btn btn-sm ${rpe === value ? `${getButtonColor(value)} border-base-content border-2` : getButtonColor(value)}`}
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
		<span class={`badge ${getRpeColor(rpe)}`}>{getRpeLabel(rpe)}</span>
		<button class="btn btn-ghost btn-sm opacity-75" onclick={() => (editMode = true)}> ✏️ </button>
	</div>
{/if}

<style>
	/* RPE Color Scale */
	.rpe-light-blue {
		background-color: #bfdbfe; /* blue-200 */
		color: #1e3a8a; /* blue-900 */
	}

	.rpe-light-green {
		background-color: #bbf7d0; /* green-200 */
		color: #14532d; /* green-900 */
	}

	.rpe-light-yellow {
		background-color: #fef08a; /* yellow-200 */
		color: #713f12; /* yellow-900 */
	}

	.rpe-orange {
		background-color: #fed7aa; /* orange-200 */
		color: #7c2d12; /* orange-900 */
	}

	.rpe-red {
		background-color: #fecaca; /* red-200 */
		color: #7f1d1d; /* red-900 */
	}

	/* Ensure good contrast for badge */
	.badge.rpe-light-blue,
	.badge.rpe-light-green,
	.badge.rpe-light-yellow,
	.badge.rpe-orange,
	.badge.rpe-red {
		border: 1px solid currentColor;
	}
</style>
