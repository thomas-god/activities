<script lang="ts">
	import { page } from '$app/state';
	import type { Snippet } from 'svelte';

	let { cta }: { cta?: Snippet } = $props();

	const classExactPath = (targetPath: string): string => {
		return page.url.pathname === targetPath ? 'active' : '';
	};

	const classPathStartWith = (targetPath: string): string => {
		return page.url.pathname.startsWith(targetPath) ? 'active' : '';
	};
</script>

<div class="flex flex-col justify-between gap-3 min-[750px]:flex-row min-[750px]:items-center">
	<div class="flex gap-3 sm:gap-6">
		<a class={`btn px-2 text-lg font-bold btn-ghost sm:text-xl ${classExactPath('/')}`} href="/"
			>Activities</a
		>
		<a
			class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classExactPath('/history')}`}
			href="/history">History</a
		>
		<a
			class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classPathStartWith('/training/metrics')}`}
			href="/training/metrics">Metrics</a
		>
		<a
			class={`btn px-2 text-[16px] font-medium btn-ghost sm:text-lg ${classPathStartWith('/training/period')}`}
			href="/training/periods">Periods</a
		>
	</div>

	{@render cta?.()}
</div>

<style>
	.active {
		border-bottom-color: var(--color-primary);
		border-bottom-width: 2px;
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
</style>
