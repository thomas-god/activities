<script lang="ts">
	import { formatDuration, localiseDateTime } from '$lib/duration';
	import EditableString from '$components/molecules/EditableString.svelte';
	import { getSportCategoryIcon, type SportCategory } from '$lib/sport';
	import type { ActivityDetails } from '$lib/api/activities';

	interface Props {
		activity: ActivityDetails;
		onEditNameCallback: (newName: string) => Promise<void>;
		onDeleteClickedCallback: () => void;
	}

	let { activity, onEditNameCallback, onDeleteClickedCallback }: Props = $props();

	let title = $derived(
		activity.name === null || activity.name === '' ? activity.sport : activity.name
	);
	let duration = $derived(formatDuration(activity.duration));

	const categoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};
</script>

<div
	class={`item mt-5 flex flex-1 items-center bg-base-100 p-3 ${categoryClass(activity.sport_category)}`}
>
	<div class={`icon ${categoryClass(activity.sport_category)}`}>
		{getSportCategoryIcon(activity.sport_category)}
	</div>
	<div class="flex flex-1 flex-col">
		<div class="mb-1 text-lg font-semibold">
			<EditableString content={title} editCallback={onEditNameCallback} />
		</div>
		<div class="text-xs font-light">
			{localiseDateTime(activity.start_time)}
		</div>
	</div>
	<div class="font-semibold sm:text-lg">
		<div>
			{duration}
		</div>
	</div>
	<div class="dropdown dropdown-end ml-2">
		<button tabindex="0" class="btn btn-circle btn-ghost btn-sm" aria-label="More options">
			⋮
		</button>
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<ul tabindex="0" class="dropdown-content menu z-[1] w-52 rounded-box bg-base-100 p-2 shadow">
			<li>
				<button onclick={onDeleteClickedCallback} class="text-error"> 🗑️ Delete Activity </button>
			</li>
		</ul>
	</div>
</div>

<style>
	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.item.cycling {
		border-left-color: var(--color-cycling);
	}

	.item.running {
		border-left-color: var(--color-running);
	}

	.item.other {
		border-left-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
	}

	.icon.cycling {
		background: var(--color-cycling-background);
		color: var(--color-cycling);
	}

	.icon.running {
		background: var(--color-running-background);
		color: var(--color-running);
	}

	.icon.other {
		background: var(--color-other-background);
		color: var(--color-other);
	}
</style>
