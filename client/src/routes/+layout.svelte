<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="mb-2 navbar bg-base-100 shadow-sm">
	<div class="flex-1">
		<a class="btn text-xl font-bold btn-ghost" href="/">Activities</a>
		<a class="btn text-lg font-medium btn-ghost" href="/training">Training</a>
	</div>
</div>

{@render children?.()}
