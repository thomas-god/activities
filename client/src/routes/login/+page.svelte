<script lang="ts">
	import { PUBLIC_APP_URL } from '$env/static/public';
	import z from 'zod';

	let email = $state('');
	const validator = z.email();
	let isValid = $derived(validator.safeParse(email).success);

	let promise: Promise<Response> | undefined = $state(undefined);

	const callbackLogin = async () => {
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

	const callbackRegister = async () => {
		if (!isValid) {
			return;
		}

		promise = fetch(`${PUBLIC_APP_URL}/api/register?email=${encodeURIComponent(email)}`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors'
		});
		await promise;
	};
</script>

{#if promise === undefined}
	<div class="sm:w-sm mx-2 mb-2 sm:mx-auto">
		<fieldset class="fieldset rounded-box border-base-300 bg-base-100 border p-4">
			<legend class="fieldset-legend">Login or register</legend>

			<label class="label" for="login-email">Email</label>
			<input
				type="email"
				class="validator input"
				placeholder="Email"
				id="login-email"
				required
				bind:value={email}
			/>

			<div class="join join-horizontal mx-auto mt-4 gap-4">
				<button
					class="join-item btn btn-primary rounded-xs"
					disabled={!isValid}
					onclick={callbackLogin}>Login</button
				>
				<button
					class="join-item btn btn-secondary rounded-xs"
					disabled={!isValid}
					onclick={callbackRegister}>Register</button
				>
			</div>
		</fieldset>
	</div>
{:else}
	{#await promise}
		<span class="loading loading-xl loading-spinner"></span>
	{:then}
		<div class="card rounded-box bg-base-100 sm:w-sm mx-2 mt-6 p-4 sm:mx-auto">
			<p>You're going to receive an email containing a magic link to login!</p>
		</div>
	{/await}
{/if}
