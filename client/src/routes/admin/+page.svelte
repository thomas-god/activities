<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import z from 'zod';
	import Navbar from '#ui/navigation/Navbar.svelte';
	import {
		createUser,
		listUsers,
		revokeUserSessions,
		setAdmin,
		setUserPassword,
		type AdminUser
	} from '#lib/api/index.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// A non-admin reaching the page (e.g. via a bookmark) is sent back home.
	// svelte-ignore state_referenced_locally
	if (data.forbidden) {
		goto(resolve('/'));
	}

	// Intentional capture of the load result as the initial state; refreshed via refresh().
	// svelte-ignore state_referenced_locally
	let users: AdminUser[] = $state(data.users ?? []);
	let onlyPasswordless = $state(false);
	const visibleUsers = $derived(
		onlyPasswordless ? users.filter((user) => !user.has_password) : users
	);

	let statusMessage: string | null = $state(null);
	let statusIsError = $state(false);
	const setStatus = (message: string, isError = false) => {
		statusMessage = message;
		statusIsError = isError;
	};

	const refresh = async () => {
		try {
			users = await listUsers();
		} catch {
			// Keep the stale list: individual actions surface their own errors.
		}
	};

	// --- Create user ---

	const emailValidator = z.email();
	let newEmail = $state('');
	let newPassword = $state('');
	let createPending = $state(false);
	const newEmailValid = $derived(emailValidator.safeParse(newEmail).success);

	const submitCreate = async () => {
		if (!newEmailValid || newPassword.length === 0 || createPending) {
			return;
		}
		createPending = true;
		try {
			const res = await createUser(newEmail, newPassword);
			if (res.status === 201) {
				newEmail = '';
				newPassword = '';
				setStatus('User created.');
				await refresh();
			} else if (res.status === 409) {
				setStatus('An account already exists for this email.', true);
			} else if (res.status === 400) {
				setStatus('Invalid email or empty password.', true);
			} else if (res.status === 403) {
				setStatus('You are not allowed to do this.', true);
			} else {
				setStatus('Something went wrong, retry later.', true);
			}
		} catch {
			setStatus('Something went wrong, retry later.', true);
		} finally {
			createPending = false;
		}
	};

	// --- Set / reset a password ---

	let passwordDialog: HTMLDialogElement;
	let passwordTarget: AdminUser | null = $state(null);
	let passwordValue = $state('');
	let passwordPending = $state(false);

	const openPasswordDialog = (user: AdminUser) => {
		passwordTarget = user;
		passwordValue = '';
		passwordDialog.showModal();
	};

	const submitPassword = async () => {
		if (!passwordTarget || passwordValue.length === 0 || passwordPending) {
			return;
		}
		passwordPending = true;
		try {
			const res = await setUserPassword(passwordTarget.user_id, passwordValue);
			if (res.status === 200) {
				setStatus(`Password updated for ${passwordTarget.email}.`);
				passwordDialog.close();
				await refresh();
			} else if (res.status === 404) {
				setStatus('This user no longer exists.', true);
			} else if (res.status === 400) {
				setStatus('The password must not be empty.', true);
			} else {
				setStatus('Something went wrong, retry later.', true);
			}
		} catch {
			setStatus('Something went wrong, retry later.', true);
		} finally {
			passwordPending = false;
		}
	};

	// --- Admin flag ---

	const toggleAdmin = async (user: AdminUser) => {
		try {
			const res = await setAdmin(user.user_id, !user.is_admin);
			if (res.status === 200) {
				await refresh();
			} else if (res.status === 409) {
				setStatus('Cannot remove the last admin.', true);
			} else if (res.status === 404) {
				setStatus('This user no longer exists.', true);
			} else {
				setStatus('Something went wrong, retry later.', true);
			}
		} catch {
			setStatus('Something went wrong, retry later.', true);
		}
	};

	// --- Revoke sessions ---

	const revokeSessions = async (user: AdminUser) => {
		try {
			const res = await revokeUserSessions(user.user_id);
			if (res.ok) {
				const body = (await res.json()) as { revoked: number };
				setStatus(`Revoked ${body.revoked} session(s) for ${user.email}.`);
			} else {
				setStatus('Something went wrong, retry later.', true);
			}
		} catch {
			setStatus('Something went wrong, retry later.', true);
		}
	};
</script>

<Navbar />

<div class="mt-4 flex flex-col gap-4">
	<h1 class="text-xl font-bold">Admin</h1>

	<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
		<legend class="fieldset-legend">Create a user</legend>
		<p class="text-xs opacity-70">
			Creates an account even when public registration is closed. The email stays unverified until
			the user confirms it.
		</p>

		<label class="label mt-2" for="admin-new-email">Email</label>
		<input id="admin-new-email" type="email" class="input validator" bind:value={newEmail} />

		<label class="label" for="admin-new-password">Password</label>
		<input id="admin-new-password" type="password" class="input" bind:value={newPassword} />

		<button
			class="btn btn-primary btn-sm mt-3 self-start"
			disabled={!newEmailValid || newPassword.length === 0 || createPending}
			onclick={submitCreate}>Create user</button
		>
	</fieldset>

	<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
		<legend class="fieldset-legend">Users ({users.length})</legend>

		<label class="label cursor-pointer justify-start gap-2">
			<input type="checkbox" class="checkbox checkbox-sm" bind:checked={onlyPasswordless} />
			<span class="label-text">Only users without a password</span>
		</label>

		<div class="mt-2 overflow-x-auto">
			<table class="table table-sm">
				<thead>
					<tr>
						<th>Email</th>
						<th>Verified</th>
						<th>Password</th>
						<th>Admin</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each visibleUsers as user (user.user_id)}
						<tr>
							<td>
								{user.email}
								{#if !user.email_valid}
									<span class="badge badge-warning badge-sm ml-1">invalid</span>
								{/if}
							</td>
							<td>{user.verified ? 'Yes' : 'No'}</td>
							<td>{user.has_password ? 'Set' : 'None'}</td>
							<td>{user.is_admin ? 'Yes' : 'No'}</td>
							<td class="flex flex-wrap justify-end gap-1">
								<button class="btn btn-ghost btn-xs" onclick={() => openPasswordDialog(user)}>
									Password
								</button>
								<button class="btn btn-ghost btn-xs" onclick={() => toggleAdmin(user)}>
									{user.is_admin ? 'Revoke admin' : 'Make admin'}
								</button>
								<button class="btn btn-ghost btn-xs" onclick={() => revokeSessions(user)}>
									Revoke sessions
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</fieldset>

	{#if statusMessage}
		<p class={statusIsError ? 'text-error' : 'text-info'}>{statusMessage}</p>
	{/if}
</div>

<dialog class="modal" bind:this={passwordDialog}>
	<div class="modal-box">
		<h3 class="text-lg font-bold">
			Set password{passwordTarget ? ` for ${passwordTarget.email}` : ''}
		</h3>

		<label class="label mt-2" for="admin-set-password">New password</label>
		<input id="admin-set-password" type="password" class="input" bind:value={passwordValue} />

		<div class="modal-action">
			<button class="btn btn-ghost" onclick={() => passwordDialog.close()}>Cancel</button>
			<button
				class="btn btn-primary"
				disabled={passwordValue.length === 0 || passwordPending}
				onclick={submitPassword}>Set password</button
			>
		</div>
	</div>
	<form method="dialog" class="modal-backdrop">
		<button>close</button>
	</form>
</dialog>
