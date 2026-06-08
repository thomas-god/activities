<script lang="ts">
	import { isNone, none, some, type Option } from '$lib/Options';

	let {
		onClickCallback,
		onSuccessCallback
	}: { onClickCallback: () => Promise<void>; onSuccessCallback: () => void } = $props();

	const setPromise = () => (promise = some(onClickCallback().then(onSuccessCallback)));
	let promise: Option<Promise<void>> = $state(none());
</script>

{#if isNone(promise)}
	<button title="Copy" class="btn join-item btn-ghost btn-xs" onclick={setPromise}>
		<img src="/icons/copy.svg" alt="Copy icon" class="inline h-5 w-5" />
	</button>
{:else}
	{#await promise}
		<button title="Copy" class="btn join-item btn-ghost btn-xs" disabled>
			<div class="loading loading-ball"></div>
		</button>
	{:then _}
		<button title="Copy" class="btn join-item btn-ghost btn-xs" onclick={() => (promise = none())}>
			<img src="/icons/check.svg" alt="Check icon" class="inline h-5 w-5" />
		</button>
	{/await}
{/if}
