<script lang="ts">
	import { PUBLIC_APP_URL } from '$env/static/public';
	import z from 'zod';

	let email = $state('');
	const validator = z.email();
	let isValid = $derived(validator.safeParse(email).success);

	let promise: Promise<Response> | undefined = $state(undefined);

	const callback = async () => {
		if (!isValid) {
			return;
		}

		promise = fetch(`${PUBLIC_APP_URL}/api/login?email=${encodeURIComponent(email)}`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors'
		});
		await promise;
	};
</script>

{#if promise === undefined}
	<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<legend class="fieldset-legend">Login</legend>

			<label class="label" for="login-email">Email</label>
			<input
				type="email"
				class="validator input"
				placeholder="Email"
				id="login-email"
				required
				bind:value={email}
			/>

			<button class="btn mt-4 btn-neutral" disabled={!isValid} onclick={callback}>Login</button>
		</fieldset>
	</div>
{:else}
	{#await promise}
		<span class="loading loading-xl loading-spinner"></span>
	{:then}
		<div class="card mx-2 mt-6 rounded-box bg-base-100 p-4 sm:mx-auto sm:w-sm">
			<p>You're going to receive an email containing a magic link to login!</p>
		</div>
	{/await}
{/if}
