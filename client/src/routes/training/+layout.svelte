<script lang="ts">
	import { page } from '$app/state';

	let { children } = $props();

	const isSelected = (targetPath: string) => {
		return page.url.pathname === targetPath ? 'tab-active' : '';
	};
</script>

<div class="mt-4 rounded-box rounded-b-none bg-base-100 p-4">
	<div role="tablist" class="tabs-lift tabs">
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
