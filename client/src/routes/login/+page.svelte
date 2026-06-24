<script lang="ts">
	import { goto } from '$app/navigation';
	import Navbar from '$components/organisms/navigation/Navbar.svelte';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { isNone, none, some, type Option } from '$lib/Options';
	import z from 'zod';

	let email = $state('');
	const validator = z.email();
	let isEmailValid = $derived(validator.safeParse(email).success);
	let password = $state('');

	let emailPromise: Promise<Response> | undefined = $state(undefined);
	let passwordPromise: Option<Promise<Response>> = $state(none());

	let authInfo = (await (
		await fetch(`${PUBLIC_APP_URL}/api/auth_info`, {
			method: 'GET'
		})
	).text()) as 'NoAuth' | 'SinglePassword' | 'EmailBased';

	const callbackPasswordLogin = async () => {
		if (password.trim().length === 0) {
			return;
		}

		passwordPromise = some(
			fetch(`${PUBLIC_APP_URL}/api/login`, {
				method: 'POST',
				credentials: 'include',
				mode: 'cors',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ password })
			}).then((res) => {
				if (res.status === 200) {
					goto('/');
				}
				return res;
			})
		);
	};

	const callbackEmailLogin = async () => {
		if (!isEmailValid) {
			return;
		}

		emailPromise = fetch(`${PUBLIC_APP_URL}/api/login?email=${encodeURIComponent(email)}`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors'
		});
		await emailPromise;
	};

	const callbackRegister = async () => {
		if (!isEmailValid) {
			return;
		}

		emailPromise = fetch(`${PUBLIC_APP_URL}/api/register?email=${encodeURIComponent(email)}`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors'
		});
		await emailPromise;
	};
</script>

<Navbar />

{#if authInfo === 'EmailBased'}
	{#if emailPromise === undefined}
		<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
			<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
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

				<div class="join mx-auto join-horizontal mt-4 gap-4">
					<button
						class="btn join-item rounded-xs btn-primary"
						disabled={!isEmailValid}
						onclick={callbackEmailLogin}>Login</button
					>
					<button
						class="btn join-item rounded-xs btn-secondary"
						disabled={!isEmailValid}
						onclick={callbackRegister}>Register</button
					>
				</div>
			</fieldset>
		</div>
	{:else}
		{#await emailPromise}
			<div class="flex w-full flex-row items-center justify-center pt-6">
				<div class="loading loading-xl loading-spinner"></div>
			</div>
		{:then}
			<div class="card mx-2 mt-6 rounded-box bg-base-100 p-4 sm:mx-auto sm:w-sm">
				<p>You're going to receive an email containing an authentication link to login!</p>
			</div>
		{/await}
	{/if}
{:else if authInfo === 'SinglePassword'}
	<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<legend class="fieldset-legend">Login</legend>

			<label class="label" for="login-email">Password</label>
			<input
				type="password"
				class="input"
				placeholder="Password"
				id="login-password"
				required
				bind:value={password}
				onkeypress={(e) => {
					if (e.key === 'Enter') {
						callbackPasswordLogin();
					}
				}}
			/>

			<div class="join mx-auto join-horizontal mt-4 gap-4">
				{#if isNone(passwordPromise)}
					<button
						class="btn join-item rounded-xs btn-primary"
						disabled={password.trim().length === 0}
						onclick={callbackPasswordLogin}>Login</button
					>
				{:else}
					{#await passwordPromise.value}
						<button class="btn join-item rounded-xs btn-primary" disabled>
							Login
							<span class="loading loading-spinner"></span>
						</button>
					{:then res}
						<div class="flex flex-col gap-3">
							<button
								class="btn join-item rounded-xs btn-primary"
								disabled={password.trim().length === 0}
								onclick={callbackPasswordLogin}>Login</button
							>
							{#if res.status === 401}
								<span class="text-error">Invalid password</span>
							{:else if res.status === 429}
								<span class="text-error">Too many attempts, retry later</span>
							{/if}
						</div>
					{/await}
				{/if}
			</div>
		</fieldset>
	</div>
{/if}
