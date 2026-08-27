<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getAuthInfo, logout, type AuthInfo } from '$lib/api/auth';
	import ThemeToggle from '$ui/shared/ThemeToggle.svelte';

	interface Cta {
		label: string;
		onClick: () => void;
	}

	let { ctas = [] }: { ctas?: Cta[] } = $props();

	let authInfo: AuthInfo | undefined = $state(undefined);
	getAuthInfo().then((info) => (authInfo = info));

	let showLogout = $derived(authInfo !== undefined && authInfo.strategy !== 'NoAuth');

	let theme = $state<'light' | 'dark'>('light');

	$effect(() => {
		const stored = localStorage.getItem('theme');
		theme =
			stored === 'light' || stored === 'dark'
				? stored
				: window.matchMedia('(prefers-color-scheme: dark)').matches
					? 'dark'
					: 'light';
	});

	const toggleTheme = () => {
		theme = theme === 'dark' ? 'light' : 'dark';
		localStorage.setItem('theme', theme);
		document.documentElement.setAttribute('data-theme', theme);
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

	// On mobile, cta buttons, the logout button, and the theme toggle collapse into a single menu.
	let mobileMenuItems = $derived([
		...ctas,
		...(showLogout ? [{ label: 'Log out', onClick: handleLogout }] : []),
		{ label: theme === 'dark' ? 'Light mode' : 'Dark mode', onClick: toggleTheme }
	]);
</script>

<div class="flex items-center justify-between gap-2">
	<div class="flex shrink gap-2 overflow-x-auto xs:gap-3 sm:gap-6">
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

	<div class="flex shrink-0 items-center gap-3">
		<div class="hidden flex-row justify-end gap-2 min-[850px]:flex">
			{#each ctas as cta, idx (cta.label)}
				<button
					class={`btn rounded-lg btn-sm sm:btn-md ${idx === 0 ? 'btn-primary' : 'btn-outline btn-primary'}`}
					onclick={cta.onClick}>+ {cta.label}</button
				>
			{/each}
		</div>
		{#if showLogout}
			<button class="btn hidden btn-ghost btn-sm min-[850px]:flex" onclick={handleLogout}
				>Log out</button
			>
		{/if}
		<ThemeToggle {theme} onToggle={toggleTheme} class="hidden min-[850px]:flex" />

		<div class="dropdown dropdown-end min-[850px]:hidden">
			<button tabindex="0" class="btn btn-outline btn-primary btn-sm" aria-label="Quick actions">
				<img src="/icons/menu.svg" class="h-5 w-5" alt="Menu icon" />
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
