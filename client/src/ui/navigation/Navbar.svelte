<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { logout } from '$lib/api/auth';
	import ThemeToggle from '$ui/shared/ThemeToggle.svelte';
	import { getTheme, persistTheme } from '$lib/contexts/theme';
	import { getAuthInfo } from '$lib/contexts/auth';
	import { isSome, none, some, type Option } from '$lib/Options';
	import {
		CalendarFold,
		ChartColumn,
		CirclePlus,
		Menu,
		MessageSquareHeart,
		NotebookPen,
		SportShoe,
		Utensils
	} from '@lucide/svelte';
	import ActivitiesUploader from './internal/ActivitiesUploader.svelte';
	import CreateTrainingNote from './internal/CreateTrainingNote.svelte';
	import FeedbackForm from './internal/FeedbackForm.svelte';
	import WeightAndNutritionForm from './internal/WeightAndNutritionForm.svelte';
	import CreateTrainingPeriod from './internal/CreateTrainingPeriod.svelte';
	import TrainingMetricFormCreate from '$ui/training_metrics/TrainingMetricFormCreate.svelte';
	import type { Scope } from '$ui/training_metrics';

	let {
		invalidateActivities = () => {},
		invalidateTrainingNotes = () => {},
		invalidateTrainingPeriods = () => {},
		invalidateTrainingMetrics = () => {}
	}: {
		invalidateActivities?: () => void;
		invalidateTrainingNotes?: () => void;
		invalidateTrainingPeriods?: () => void;
		invalidateTrainingMetrics?: () => void;
	} = $props();

	let authInfo = getAuthInfo();
	let showLogout = $derived(
		isSome(authInfo) && authInfo.value !== undefined && authInfo.value.strategy !== 'NoAuth'
	);

	let theme = getTheme();
	const toggleTheme = () => {
		theme.variant = theme.variant === 'dark' ? 'light' : 'dark';
		persistTheme(theme);
	};

	const classExactPath = (targetPath: string): string => {
		return page.url.pathname === targetPath ? 'active' : '';
	};

	const classPathStartWith = (targetPath: string): string => {
		return page.url.pathname.startsWith(targetPath) ? 'active' : '';
	};

	const handleLogout = async () => {
		await logout();
		goto(resolve('/login'));
	};

	// On mobile the logout button, and the theme toggle collapse into a single menu.
	let mobileMenuItems = $derived([
		...(showLogout ? [{ label: 'Log out', onClick: handleLogout }] : []),
		{ label: theme.variant === 'dark' ? 'Light mode' : 'Dark mode', onClick: toggleTheme }
	]);

	let addItemMenuBtn: HTMLButtonElement;

	const activitiesUploadedCallback = () => {
		invalidateActivities();
	};
	const newTrainingNoteCallback = () => {
		invalidateTrainingNotes();
	};
	const createTrainingPeriodCallback = () => {
		invalidateTrainingPeriods();
	};
	const createTrainingMetricCallback = () => {
		invalidateTrainingMetrics();
	};

	let trainingPeriod: Option<string> = $derived(
		page.url.pathname.startsWith('/training/period/')
			? some(page.url.pathname.replace('/training/period/', ''))
			: none()
	);
	let trainingPeriodScope: Scope = $derived(
		isSome(trainingPeriod) ? { kind: 'period', periodId: trainingPeriod.value } : { kind: 'global' }
	);

	let quickMenuDialog: HTMLDialogElement;
	const quickMenuItems = ['activity', 'feedback', 'w&n', 'note', 'period', 'metric'] as const;
	const indexByItem = new Map(quickMenuItems.entries().map(([i, v]) => [v, i]));
	let quickMenuIndex = $state(0);
	const nextMenuItem = () => {
		if (quickMenuIndex === quickMenuItems.length - 1) {
			quickMenuIndex = 0;
			return;
		}
		quickMenuIndex = Math.min(quickMenuIndex + 1, quickMenuItems.length - 1);
	};
	const previousMenuItem = () => {
		if (quickMenuIndex === 0) {
			quickMenuIndex = quickMenuItems.length - 1;
			return;
		}
		quickMenuIndex = Math.max(quickMenuIndex - 1, 0);
	};
	const setQuickMenuIndex = (item: (typeof quickMenuItems)[number]) => {
		quickMenuDialog.show();
		quickMenuIndex = indexByItem.get(item)!;
	};
	let selectedItem = $derived(quickMenuItems[quickMenuIndex]);
</script>

<div class="flex items-center justify-between gap-2">
	<div class="flex shrink gap-1 overflow-x-auto xs:gap-3 sm:gap-6">
		<a
			class={`btn shrink-0 btn-ghost px-1 text-[16px] font-bold xs:px-2 xs:text-lg sm:text-xl ${classExactPath('/')}`}
			href={resolve('/')}>Activities</a
		>
		<a
			class={`btn shrink-0 btn-ghost px-1 text-[15px] font-medium xs:px-2 xs:text-[16px] sm:text-lg ${classExactPath('/history')}`}
			href={resolve('/history')}>History</a
		>
		<a
			class={`btn shrink-0 btn-ghost px-1 text-[15px] font-medium xs:px-2 xs:text-[16px] sm:text-lg ${classPathStartWith('/training/metrics')}`}
			href={resolve('/training/metrics')}>Metrics</a
		>
		<a
			class={`btn shrink-0 btn-ghost px-1 text-[15px] font-medium xs:px-2 xs:text-[16px] sm:text-lg ${classPathStartWith('/training/period')}`}
			href={resolve('/training/periods')}>Periods</a
		>
	</div>

	<div class="flex shrink-0 items-center gap-0 min-[400px]:gap-2">
		<button
			bind:this={addItemMenuBtn}
			class="btn btn-ghost btn-primary btn-xs min-[400px]:btn-sm"
			popovertarget="add-item-menu"
			style="anchor-name:--anchor-add-item"
		>
			<CirclePlus class="size-5" />
		</button>
		<div
			popover
			id="add-item-menu"
			style="position-anchor:--anchor-add-item"
			class="menu dropdown w-52 rounded-box bg-base-100 shadow-sm"
		>
			<ul>
				<li>
					<button onclick={() => setQuickMenuIndex('activity')}>
						<SportShoe class="size-4" />
						Activity
					</button>
					<button onclick={() => setQuickMenuIndex('feedback')}>
						<MessageSquareHeart class="size-4" />
						Feedback
					</button>
					<button onclick={() => setQuickMenuIndex('w&n')}>
						<Utensils class="size-4" />
						Weight and nutrition
					</button>
					<button onclick={() => setQuickMenuIndex('note')}>
						<NotebookPen class="size-4" />
						Training note
					</button>
					<button onclick={() => setQuickMenuIndex('period')}>
						<CalendarFold class="size-4" />
						Training period
					</button>
					<button onclick={() => setQuickMenuIndex('metric')}>
						<ChartColumn class="size-4" />
						Training metric
					</button>
				</li>
			</ul>
		</div>

		{#if showLogout}
			<button class="btn hidden btn-ghost btn-sm min-[400px]:flex" onclick={handleLogout}
				>Log out</button
			>
		{/if}
		<ThemeToggle {theme} onToggle={toggleTheme} class="hidden min-[850px]:flex" />

		<div class="dropdown dropdown-end min-[850px]:hidden">
			<button
				tabindex="0"
				class="btn btn-ghost btn-primary btn-xs [min-850px]:btn-sm"
				aria-label="Quick actions"
			>
				<Menu class="size-5" />
			</button>
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<ul tabindex="0" class="menu dropdown-content z-1 w-48 rounded-box bg-base-100 p-2 shadow">
				{#each mobileMenuItems as item (item.label)}
					<li><button onclick={item.onClick}>{item.label}</button></li>
				{/each}
			</ul>
		</div>
	</div>
</div>

<dialog class="modal" bind:this={quickMenuDialog}>
	<div class="overflow-y-none modal-box h-150 w-[90%] max-w-lg">
		<form method="dialog">
			<button class="btn absolute top-2 right-2 btn-circle btn-ghost btn-sm">✕</button>
		</form>
		{#if selectedItem === 'activity'}
			<ActivitiesUploader
				{activitiesUploadedCallback}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{:else if selectedItem === 'feedback'}
			<FeedbackForm
				callback={invalidateTrainingMetrics}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{:else if selectedItem === 'w&n'}
			<WeightAndNutritionForm
				callback={invalidateTrainingMetrics}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{:else if selectedItem === 'note'}
			<CreateTrainingNote
				callback={newTrainingNoteCallback}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{:else if selectedItem === 'period'}
			<CreateTrainingPeriod
				callback={createTrainingPeriodCallback}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{:else if selectedItem === 'metric'}
			<TrainingMetricFormCreate
				callback={createTrainingMetricCallback}
				scope={trainingPeriodScope}
				previous={some(previousMenuItem)}
				next={some(nextMenuItem)}
			/>
		{/if}
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>

<style>
	.active {
		border-bottom-color: var(--color-primary);
		border-bottom-width: 2px;
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
</style>
