<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="page-container">
	{@render children?.()}
</div>

<style>
	.page-container {
		max-width: 1350px;
		margin: 0 auto;
		padding: 12px 8px;

		@media (min-width: 640px) {
			padding: 20px;
		}
	}
</style>
