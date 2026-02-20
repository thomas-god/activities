<script lang="ts">
	import { localiseDateTime } from '$lib/duration';
	import EditableActivityName from '$components/molecules/EditableActivityName.svelte';
	import { getSportCategoryIcon, sportDisplay, type SportCategory } from '$lib/sport';
	import type { Activity } from '$lib/api/activities';

	interface Props {
		activity: Activity;
		onEditNameCallback: (newName: string) => Promise<void>;
		onDeleteClickedCallback: () => void;
		compact?: boolean;
	}

	let { activity, onEditNameCallback, onDeleteClickedCallback, compact = false }: Props = $props();

	let title = $derived(
		activity.name === null || activity.name === '' ? sportDisplay(activity.sport) : activity.name
	);

	const categoryClass = (category: SportCategory | null): string => {
		if (category === 'Running') {
			return 'running';
		}
		if (category === 'Cycling') {
			return 'cycling';
		}
		return 'other';
	};

	const callback = async (newName: string) => {
		title = newName;
		onEditNameCallback(newName);
	};
</script>

<div
	class={`item flex flex-1 items-center bg-base-100 p-3 ${categoryClass(activity.sport_category)} ${compact ? 'compact' : ''}`}
>
	<div class={`icon ${categoryClass(activity.sport_category)}`}>
		<img
			src={`/icons/${getSportCategoryIcon(activity.sport_category)}`}
			class="h-8 w-8"
			alt="Sport icon"
		/>
	</div>
	<div class="flex flex-1 flex-col">
		<div class="mb-1 text-lg font-semibold">
			<EditableActivityName name={title} editCallback={callback} />
		</div>
		<div class="text-xs font-light">
			{localiseDateTime(activity.start_time)}
			·
			{sportDisplay(activity.sport)}

			<div class="dropdown dropdown-end ml-0">
				<button tabindex="0" class="btn btn-ghost btn-xs" aria-label="More options">
					<img src="/icons/menu.svg" class="h-4 w-4" alt="Menu icon" />
				</button>
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<ul
					tabindex="0"
					class="dropdown-content menu z-[1] w-52 rounded-box bg-base-100 p-2 shadow"
				>
					<li>
						<button onclick={onDeleteClickedCallback} class="text-error">
							🗑️ Delete Activity
						</button>
					</li>
				</ul>
			</div>
		</div>
	</div>
</div>

<style>
	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.item.compact {
		border-radius: 0;
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
