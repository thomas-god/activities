<script lang="ts">
	import { resolve } from '$app/paths';
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
		<a
			class={`btn btn-ghost px-2 text-lg font-bold sm:text-xl ${classExactPath('/')}`}
			href={resolve('/')}>Activities</a
		>
		<a
			class={`btn btn-ghost px-2 text-[16px] font-medium sm:text-lg ${classExactPath('/history')}`}
			href={resolve('/history')}>History</a
		>
		<a
			class={`btn btn-ghost px-2 text-[16px] font-medium sm:text-lg ${classPathStartWith('/training/metrics')}`}
			href={resolve('/training/metrics')}>Metrics</a
		>
		<a
			class={`btn btn-ghost px-2 text-[16px] font-medium sm:text-lg ${classPathStartWith('/training/period')}`}
			href={resolve('/training/periods')}>Periods</a
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
