<script lang="ts">
	import { page } from '$app/state';

	let { children } = $props();

	const isSelected = (targetPath: string) => {
		return page.url.pathname === targetPath ? 'tab-active' : '';
	};
</script>

<div class="my-4 rounded-box bg-base-100">
	<div role="tablist" class="tabs flex flex-row justify-center gap-2 tabs-box px-3">
		<a
			role="tab"
			class={`tab fieldset-legend ${isSelected('/training/metrics')}`}
			href="/training/metrics">Metrics</a
		>
		<a
			role="tab"
			class={`tab fieldset-legend ${isSelected('/training/periods')}`}
			href="/training/periods">Periods</a
		>
	</div>
</div>
<div>
	{@render children?.()}
</div>
