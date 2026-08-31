<script lang="ts">
	import { isNone, none, some, type Option } from '$lib/Options';
	import { Check, CopyPlus } from '@lucide/svelte';

	let {
		onClickCallback,
		onSuccessCallback
	}: { onClickCallback: () => Promise<void>; onSuccessCallback: () => void } = $props();

	const setPromise = () => (promise = some(onClickCallback().then(onSuccessCallback)));
	let promise: Option<Promise<void>> = $state(none());
</script>

{#if isNone(promise)}
	<button title="Copy" class="btn join-item btn-ghost btn-xs" onclick={setPromise}>
		<CopyPlus class="size-5" />
	</button>
{:else}
	{#await promise}
		<button title="Copy" class="btn join-item btn-ghost btn-xs" disabled>
			<div class="loading loading-ball"></div>
		</button>
	{:then _}
		<button title="Copy" class="btn join-item btn-ghost btn-xs" onclick={() => (promise = none())}>
			<Check class="size-5" />
		</button>
	{/await}
{/if}
