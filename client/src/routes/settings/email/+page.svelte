<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import z from 'zod';
	import { isSome, none, some, type Option } from '#lib/Options.js';
	import { getAuthInfo } from '#lib/contexts/auth.js';
	import Navbar from '#ui/navigation/Navbar.svelte';
	import {
		changePassword,
		fetchUserInfo,
		registerPassword,
		requestEmailChange,
		validateEmailChange,
		type UserInfo
	} from '#lib/api/index.js';

	let { data } = $props();

	let authInfo = getAuthInfo();
	const authInfoValue = $derived(isSome(authInfo) && authInfo.value ? authInfo.value : undefined);
	const emailAuthEnabled = $derived(authInfoValue?.auth_methods?.email ?? false);
	const passwordAuthEnabled = $derived(authInfoValue?.auth_methods?.password ?? false);

	// Account management endpoints only exist for the multi-user strategy.
	const isMultiUser = $derived(authInfoValue?.strategy === 'MultiUser');
	// Intentional one-shot redirect at mount time (the strategy is fixed for the session).
	// svelte-ignore state_referenced_locally
	if (!isMultiUser) {
		goto(resolve('/'));
	}

	// Intentional capture of the load result as the initial state; refreshed via refreshUserInfo().
	// svelte-ignore state_referenced_locally
	let userInfo: UserInfo | null = $state(data.userInfo);

	const refreshUserInfo = async (): Promise<void> => {
		try {
			userInfo = await fetchUserInfo();
		} catch {
			// Keep the stale info on refresh failure: actions already surface errors.
		}
	};

	// --- Email change request ---

	const emailValidator = z.email();
	let newEmail = $state('');
	let isEmailValid = $derived(emailValidator.safeParse(newEmail).success);
	let emailChangePending = $state(false);
	let emailChangeError: string | null = $state(null);
	let emailChangeInfo: string | null = $state(null);

	const submitEmailChange = async (): Promise<void> => {
		if (!isEmailValid || emailChangePending) {
			return;
		}
		emailChangePending = true;
		emailChangeError = null;
		emailChangeInfo = null;
		try {
			const res = await requestEmailChange(newEmail);
			if (res.status === 202) {
				// Verification flow: the change takes effect once the emailed link is opened.
				emailChangeInfo = `A verification link was sent to ${newEmail}. Your email address will be updated once you open it.`;
				newEmail = '';
			} else if (res.status === 200) {
				emailChangeInfo = 'Your email address has been updated.';
				newEmail = '';
				await refreshUserInfo();
			} else if (res.status === 400) {
				emailChangeError = 'Invalid email address';
			} else if (res.status === 429) {
				emailChangeError = 'Too many attempts, retry later';
			} else if (res.status === 503) {
				emailChangeError = 'Service unavailable, retry later';
			} else {
				emailChangeError = 'Something went wrong, retry later';
			}
		} catch {
			emailChangeError = 'Something went wrong, retry later';
		} finally {
			emailChangePending = false;
		}
	};

	// --- Verification link (the email link lands on /settings/email#token) ---
	//
	// The token is a URL fragment (never sent to the server), so this page must always be
	// rendered client-side to read it from `window.location.hash`.

	let validatePromise: Option<Promise<Response>> = $state(none());
	let validatedEmail: string | null = $state(null);

	if (typeof window !== 'undefined' && window.location.hash.length > 1) {
		const token = window.location.hash.slice(1);
		validatePromise = some(
			validateEmailChange(token).then(async (res) => {
				// Consume the fragment so reloading the page doesn't replay the validation.
				history.replaceState(null, '', window.location.pathname);
				if (res.status === 200) {
					res.json().then((body: { email?: string }) => {
						validatedEmail = body.email ?? null;
					});
					await refreshUserInfo();
				}
				return res;
			})
		);
	}

	// --- Password management ---

	let currentPassword = $state('');
	let newPassword = $state('');
	let passwordPending = $state(false);
	let passwordError: string | null = $state(null);
	let passwordInfo: string | null = $state(null);

	const submitPassword = async (): Promise<void> => {
		if (!userInfo || !newPassword || passwordPending) {
			return;
		}
		passwordPending = true;
		passwordError = null;
		passwordInfo = null;
		try {
			const res = userInfo.has_password
				? await changePassword(currentPassword, newPassword)
				: await registerPassword(newPassword);
			if (res.status === 200) {
				passwordInfo = userInfo.has_password
					? 'Your password has been updated.'
					: 'Your password has been set.';
				currentPassword = '';
				newPassword = '';
				await refreshUserInfo();
			} else if (res.status === 400) {
				passwordError = 'A password is required';
			} else if (res.status === 401) {
				passwordError = 'Current password is incorrect';
			} else if (res.status === 403) {
				passwordError = 'Password authentication is disabled';
			} else if (res.status === 409) {
				passwordError = 'You already have a password';
				await refreshUserInfo();
			} else if (res.status === 429) {
				passwordError = 'Too many attempts, retry later';
			} else if (res.status === 503) {
				passwordError = 'Service unavailable, retry later';
			} else {
				passwordError = 'Something went wrong, retry later';
			}
		} catch {
			passwordError = 'Something went wrong, retry later';
		} finally {
			passwordPending = false;
		}
	};
</script>

<Navbar />

{#if !isMultiUser}
	<!-- Redirecting away; nothing to render. -->
{:else if userInfo === null}
	<div class="card mx-2 mt-6 rounded-box bg-base-100 p-4 sm:mx-auto sm:w-sm">
		<p class="text-error">Could not load your account information.</p>
		<button class="btn mt-4 btn-primary" onclick={refreshUserInfo}>Retry</button>
	</div>
{:else if isSome(validatePromise)}
	{#await validatePromise.value}
		<div class="mx-auto mt-12 flex w-full justify-center">
			<span class="loading loading-xl loading-spinner"></span>
		</div>
	{:then res}
		<div class="card mx-2 mt-6 rounded-box bg-base-100 p-4 sm:mx-auto sm:w-sm">
			{#if res.status === 200}
				<p class="text-success">
					Your new email address{validatedEmail ? ` (${validatedEmail})` : ''} is confirmed.
				</p>
			{:else if res.status === 409}
				<p class="text-error">This email address is now used by another account.</p>
			{:else if res.status === 401}
				<p class="text-error">This verification link is invalid or has expired.</p>
			{:else}
				<p class="text-error">Something went wrong, retry later.</p>
			{/if}
		</div>
	{/await}
{:else}
	<div class="mx-2 mb-2 sm:mx-auto sm:w-sm">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<legend class="fieldset-legend">Email</legend>

			<div class="flex items-center justify-between gap-2">
				<p class="text-sm font-bold">{userInfo.email}</p>
				<span class={`badge badge-sm ${userInfo.verified ? 'badge-success' : 'badge-warning'}`}>
					{userInfo.verified ? 'Verified' : 'Unverified'}
				</span>
			</div>

			<label class="label mt-4" for="settings-new-email">New email</label>
			<input
				type="email"
				class="validator input"
				placeholder="New email"
				id="settings-new-email"
				required
				bind:value={newEmail}
			/>

			<div class="join mx-auto mt-4 join-horizontal gap-4">
				<button
					class="btn join-item rounded-xs btn-primary"
					disabled={emailChangePending || !isEmailValid}
					onclick={submitEmailChange}>Change email</button
				>
			</div>

			{#if emailChangeError}
				<p class="text-error">{emailChangeError}</p>
			{:else if emailChangeInfo}
				<p class="text-info">{emailChangeInfo}</p>
			{:else if emailAuthEnabled}
				<p class="text-xs opacity-70">
					A verification link will be sent to the new address: the change only takes effect once you
					open it.
				</p>
			{:else}
				<p class="text-xs opacity-70">The new address will be marked as unverified.</p>
			{/if}
		</fieldset>

		{#if passwordAuthEnabled}
			<fieldset class="mt-4 fieldset rounded-box border border-base-300 bg-base-100 p-4">
				<legend class="fieldset-legend"
					>{userInfo.has_password ? 'Password' : 'Set a password'}</legend
				>

				{#if userInfo.has_password}
					<label class="label" for="settings-current-password">Current password</label>
					<input
						type="password"
						class="input"
						placeholder="Current password"
						id="settings-current-password"
						required
						bind:value={currentPassword}
					/>
				{/if}

				<label class="label" for="settings-new-password">
					{userInfo.has_password ? 'New password' : 'Password'}
				</label>
				<input
					type="password"
					class="input"
					placeholder="New password"
					id="settings-new-password"
					required
					bind:value={newPassword}
					onkeypress={(e) => {
						if (e.key === 'Enter') {
							submitPassword();
						}
					}}
				/>

				<div class="join mx-auto mt-4 join-horizontal gap-4">
					<button
						class="btn join-item rounded-xs btn-primary"
						disabled={passwordPending ||
							newPassword.length === 0 ||
							(userInfo.has_password && currentPassword.length === 0)}
						onclick={submitPassword}
						>{userInfo.has_password ? 'Change password' : 'Set password'}</button
					>
				</div>

				{#if passwordError}
					<p class="text-error">{passwordError}</p>
				{:else if passwordInfo}
					<p class="text-info">{passwordInfo}</p>
				{/if}
			</fieldset>
		{/if}
	</div>
{/if}
