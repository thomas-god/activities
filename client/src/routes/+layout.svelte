<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="navbar bg-base-100 mb-2 shadow-sm">
	<div class="flex-1">
		<a class="btn btn-ghost text-xl font-bold" href="/">Activities</a>
		<a class="btn btn-ghost text-lg font-medium" href="/training">Training</a>
	</div>
</div>

{@render children?.()}
