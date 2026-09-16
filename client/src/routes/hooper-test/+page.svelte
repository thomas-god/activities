<script lang="ts">
	import HooperIndexForm from '$ui/hooper_index/HooperIndexForm.svelte';

	// Temporary page used to manually exercise the Hooper index component.
	// Not linked from anywhere and not wired to the rest of the app.
</script>

<div class="mx-auto flex max-w-md flex-col gap-4 pt-6 pb-10">
	<h1 class="text-xl font-semibold">Hooper index</h1>
	<p class="text-sm italic opacity-60">Temporary test page — not wired to the app.</p>

	<HooperIndexForm />
</div>
