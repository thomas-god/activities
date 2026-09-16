<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { logout } from '$lib/api/auth';
	import ThemeToggle from '$ui/shared/ThemeToggle.svelte';
	import { getTheme, persistTheme } from '$lib/contexts/theme';
	import { getAuthInfo } from '$lib/contexts/auth';
	import { isSome } from '$lib/Options';
	import { CirclePlus, Menu } from '@lucide/svelte';

	interface Cta {
		label: string;
		onClick: () => void;
	}

	let { ctas = [] }: { ctas?: Cta[] } = $props();

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
				{#each ctas as cta (cta.label)}
					<li>
						<button
							onclick={() => {
								addItemMenuBtn.click();
								cta.onClick();
							}}
						>
							{cta.label}
						</button>
					</li>
				{/each}
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

<style>
	.active {
		border-bottom-color: var(--color-primary);
		border-bottom-width: 2px;
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
</style>
