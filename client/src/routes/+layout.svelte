<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="mb-2 navbar bg-base-100 shadow-sm">
	<div class="flex-1">
		<a class="btn text-xl btn-ghost" href="/">Activities</a>
	</div>
</div>

{@render children?.()}
