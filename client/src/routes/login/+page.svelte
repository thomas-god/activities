<script lang="ts">
	import { goto } from '$app/navigation';
	import Navbar from '#ui/navigation/Navbar.svelte';
	import { PUBLIC_APP_URL } from '$app/env/public';
	import { isNone, isSome, none, some, type Option } from '#lib/Options.js';
	import z from 'zod';
	import { resolve } from '$app/paths';
	import { fetchAuthInfo, type AuthInfo, type MultiUserAuthRequest } from '#lib/api/index.js';

	let email = $state('');
	const validator = z.email();
	let isEmailValid = $derived(validator.safeParse(email).success);
	let password = $state('');

	let passwordPromise: Option<Promise<Response>> = $state(none());

	// The auth-link token is a URL fragment /login#token
	const authLinkToken: Option<string> =
		window.location.hash.length > 1 ? some(window.location.hash.slice(1)) : none();

	const authLinkPromise: Option<Promise<Response>> = isSome(authLinkToken)
		? some(
				fetch(`${PUBLIC_APP_URL}/api/login/validate`, {
					method: 'POST',
					credentials: 'include',
					mode: 'cors',
					headers: {
						'Content-Type': 'application/json'
					},
					body: JSON.stringify({ token: authLinkToken.value })
				}).then((res) => {
					if (res.status === 200) {
						goto(resolve('/'));
					}
					return res;
				})
			)
		: none();

	const authInfo: AuthInfo = await fetchAuthInfo();
	// Password-based auth is always enabled in multi-user mode; email-based auth is optional. Both
	// default to false for single-user strategies, where `auth_methods` is absent.
	const emailAuthEnabled = authInfo.auth_methods?.email ?? false;
	const registrationOpen = authInfo.registration_open ?? false;

	let multiUserLoginError: string | null = $state(null);
	let multiUserLoginInfo: string | null = $state(null);
	let multiUserRequestPending = $state(false);

	const authLinkSentMessage =
		"You're going to receive an email containing an authentication link to login!";

	const postJson = (path: string, body: object): Promise<Response> =>
		fetch(`${PUBLIC_APP_URL}${path}`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(body)
		});

	const submitMultiUser = async (
		path: string,
		body: MultiUserAuthRequest,
		onSuccess: () => void | Promise<void>
	): Promise<void> => {
		multiUserLoginError = null;
		multiUserLoginInfo = null;
		multiUserRequestPending = true;
		try {
			const res = await postJson(path, body);
			if (res.status === 200) {
				await onSuccess();
			} else if (res.status === 401) {
				// Single, uniform response for unknown account / wrong password / account without a
				// password, so the client cannot (and must not) tell them apart.
				multiUserLoginError = 'Invalid login';
			} else if (res.status === 400) {
				multiUserLoginError = 'A password is required';
			} else if (res.status === 429) {
				multiUserLoginError = 'Too many attempts, retry later';
			} else if (res.status === 503) {
				multiUserLoginError = 'Service unavailable, retry later';
			} else {
				multiUserLoginError = 'Something went wrong, retry later';
			}
		} catch {
			multiUserLoginError = 'Something went wrong, retry later';
		} finally {
			multiUserRequestPending = false;
		}
	};

	const callbackMultiUserLogin = (): Promise<void> => {
		if (!isEmailValid || password.trim().length === 0) {
			return Promise.resolve();
		}
		const body: MultiUserAuthRequest = { email, password };
		return submitMultiUser('/api/login', body, () => goto(resolve('/')));
	};

	// Only reachable when email-based auth is enabled: omitting the password requests an
	// authentication link instead of logging in directly.
	const callbackRequestAuthLink = (): Promise<void> => {
		if (!isEmailValid) {
			return Promise.resolve();
		}
		return submitMultiUser('/api/login', { email } satisfies MultiUserAuthRequest, () => {
			multiUserLoginInfo = authLinkSentMessage;
		});
	};

	const callbackRegister = (): Promise<void> => {
		if (!isEmailValid || password.trim().length === 0) {
			return Promise.resolve();
		}
		return submitMultiUser('/api/register', { email, password }, () => {
			multiUserLoginInfo = emailAuthEnabled
				? 'Account created. You can log in with your password, or use the link sent to your inbox.'
				: 'Account created. You can now log in with your password.';
		});
	};

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
					goto(resolve('/'));
				}
				return res;
			})
		);
	};
</script>

<Navbar />

{#if isSome(authLinkPromise)}
	{#await authLinkPromise.value}
		<div class="mx-auto mt-12 flex w-full justify-center">
			<span class="loading loading-xl loading-spinner"></span>
		</div>
	{:then res}
		{#if res.status !== 200}
			<div class="card mx-2 mt-6 rounded-box bg-base-100 p-4 sm:mx-auto sm:w-sm">
				<p class="text-error">This login link is invalid or has expired.</p>
			</div>
		{/if}
	{/await}
{:else if authInfo.strategy === 'MultiUser'}
	<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<legend class="fieldset-legend">
				{registrationOpen ? 'Login or register' : 'Login'}
			</legend>

			<label class="label" for="login-email">Email</label>
			<input
				type="email"
				class="validator input"
				placeholder="Email"
				id="login-email"
				required
				bind:value={email}
			/>
			{#if !emailAuthEnabled}
				<p class="mt-1 text-xs opacity-70">
					Your email address is only used as your sign-in identifier, you won't receive emails on
					it.
				</p>
			{/if}

			<label class="label" for="login-password">Password</label>
			<input
				type="password"
				class="input"
				placeholder="Password"
				id="login-password"
				required
				bind:value={password}
				onkeypress={(e) => {
					if (e.key === 'Enter') {
						callbackMultiUserLogin();
					}
				}}
			/>

			<div class="join mx-auto mt-4 join-horizontal gap-4">
				<button
					class="btn join-item rounded-xs btn-primary"
					disabled={multiUserRequestPending || !isEmailValid || password.trim().length === 0}
					onclick={callbackMultiUserLogin}>Login</button
				>
				{#if registrationOpen}
					<button
						class="btn join-item rounded-xs btn-secondary"
						disabled={multiUserRequestPending || !isEmailValid || password.trim().length === 0}
						onclick={callbackRegister}>Register</button
					>
				{/if}
			</div>

			{#if emailAuthEnabled}
				<button
					class="btn mt-2 w-full btn-ghost btn-sm"
					disabled={multiUserRequestPending || !isEmailValid}
					onclick={callbackRequestAuthLink}>Email me a link instead</button
				>
			{/if}

			{#if multiUserRequestPending}
				<div class="flex w-full justify-center pt-2">
					<span class="loading loading-spinner"></span>
				</div>
			{/if}
			{#if multiUserLoginError}
				<p class="mt-2 text-error">{multiUserLoginError}</p>
			{/if}
			{#if multiUserLoginInfo}
				<p class="mt-2">{multiUserLoginInfo}</p>
			{/if}
		</fieldset>
	</div>
{:else if authInfo.strategy === 'SinglePassword'}
	<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<legend class="fieldset-legend">Login</legend>

			<label class="label" for="login-password">Password</label>
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

			<div class="join mx-auto mt-4 join-horizontal gap-4">
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
