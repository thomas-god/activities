<script lang="ts">
	import { page } from '$app/state';
	import '../app.css';

	let { children } = $props();

	const activeClass = (targetPath: string): string => {
		return page.url.pathname === targetPath ? 'active' : '';
	};
</script>

<div class="mt-4 flex flex-col justify-between gap-3 px-4 sm:flex-row sm:items-center">
	<div class="flex gap-6">
		<a class={`btn px-0 text-xl font-bold btn-ghost ${activeClass('/')}`} href="/">Activities</a>
		<a class={`btn px-0 text-lg font-medium btn-ghost ${activeClass('/history')}`} href="/history"
			>History</a
		>
		<a class={`btn px-0 text-lg font-medium btn-ghost ${activeClass('/training')}`} href="/training"
			>Metrics</a
		>
	</div>
	{#if page.url.pathname === '/'}
		<div class="mx-3">
			<button class="btn w-full rounded-lg btn-primary">+ Add activities</button>
		</div>
	{/if}
</div>

{@render children?.()}

<style>
	.active {
		border-bottom-color: var(--color-primary);
		border-bottom-width: 2px;
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
</style>
