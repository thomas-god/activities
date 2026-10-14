<script lang="ts">
	import z from 'zod';
	import {
		createUser,
		listUsers,
		revokeUserSessions,
		setAdmin,
		setUserEmail,
		setUserPassword,
		type AdminUser
	} from '#lib/api/index.js';
	import type { AdminInfo } from './+page';
	import { isSome, map, unwrapOr, type Option } from '#lib/Options.ts';
	import { CircleX, Shield, ShieldX, SquarePen } from '@lucide/svelte';

	let { adminInfo }: { adminInfo: Option<AdminInfo> } = $props();

	let users = $derived(
		unwrapOr(
			map(adminInfo, (info) => info.users),
			[]
		)
	);

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
	const clearStatus = () => {
		statusMessage = null;
		statusIsError = false;
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

	// svelte-ignore non_reactive_update
	let passwordDialog: HTMLDialogElement;
	let passwordTarget: AdminUser | null = $state(null);
	let passwordValue = $state('');
	let passwordPending = $state(false);

	const openPasswordDialog = (user: AdminUser) => {
		passwordTarget = user;
		passwordValue = '';
		clearStatus();
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
				setStatus(
					`Password updated for ${passwordTarget.email === null ? passwordTarget.user_id : passwordTarget.email}.`
				);
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

	// --- Set / reset email address ---

	// svelte-ignore non_reactive_update
	let emailDialog: HTMLDialogElement;
	let emailTarget: AdminUser | null = $state(null);
	let emailValue = $state('');
	let emailPending = $state(false);

	const openEmailDialog = (user: AdminUser) => {
		emailTarget = user;
		emailValue = '';
		clearStatus();
		emailDialog.showModal();
	};

	const submitEmail = async () => {
		if (!emailTarget || emailValue.length === 0 || emailPending) {
			return;
		}
		emailPending = true;
		try {
			const res = await setUserEmail(emailTarget.user_id, emailValue);
			if (res.status === 200) {
				setStatus(
					`Email updated for ${emailTarget.email === null ? emailTarget.user_id : emailTarget.email}.`
				);
				emailDialog.close();
				await refresh();
			} else if (res.status === 404) {
				setStatus('This user no longer exists.', true);
			} else if (res.status === 400) {
				setStatus('Invalid email address.', true);
			} else if (res.status === 409) {
				setStatus('Email address already taken by another user.', true);
			} else {
				setStatus('Something went wrong, retry later.', true);
			}
		} catch {
			setStatus('Something went wrong, retry later.', true);
		} finally {
			emailPending = false;
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

{#if isSome(adminInfo)}
	<div class="mt-4 flex flex-col gap-4">
		<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
			<h1 class="text-xl font-bold">Users administration</h1>

			<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
				<legend class="fieldset-legend">Create a user</legend>
				<p class="text-xs opacity-70">
					Creates an account even when public registration is closed. The email stays unverified
					until the user confirms it.
				</p>

				<label class="label mt-2" for="admin-new-email">Email</label>
				<input id="admin-new-email" type="email" class="validator input" bind:value={newEmail} />

				<label class="label" for="admin-new-password">Password</label>
				<input id="admin-new-password" type="password" class="input" bind:value={newPassword} />

				<div class="join mx-auto w-full join-horizontal gap-4">
					<button
						class="btn mt-3 self-start btn-primary btn-sm"
						disabled={!newEmailValid || newPassword.length === 0 || createPending}
						onclick={submitCreate}>Create user</button
					>
				</div>
			</fieldset>

			<fieldset class="fieldset rounded-box border border-base-300 bg-base-100 p-4">
				<legend class="fieldset-legend">Users ({users.length})</legend>

				<label class="label cursor-pointer justify-start gap-2">
					<input type="checkbox" class="checkbox checkbox-sm" bind:checked={onlyPasswordless} />
					<span class="label-text">Users without a password</span>
				</label>

				<div class="mt-2 overflow-x-auto">
					<table class="table table-sm">
						<thead>
							<tr>
								<th>User id</th>
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
									<td>{user.user_id}</td>
									<td>
										{user.email === null ? 'no email' : user.email}
										{#if !user.email_valid}
											<span class="ml-1 badge badge-sm badge-warning">invalid</span>
										{/if}
									</td>
									<td>{user.verified ? 'Yes' : 'No'}</td>
									<td>{user.has_password ? 'Set' : 'None'}</td>
									<td>{user.is_admin ? 'Yes' : 'No'}</td>
									<td class="flex flex-wrap gap-1">
										<button class="btn btn-xs" onclick={() => openEmailDialog(user)}>
											<SquarePen class="size-3" />
											Email
										</button>
										<button class="btn btn-xs" onclick={() => openPasswordDialog(user)}>
											<SquarePen class="size-3" />
											Password
										</button>
										<button class="btn btn-xs" onclick={() => toggleAdmin(user)}>
											{#if user.is_admin}
												<ShieldX class="size-3" />
												Revoke admin
											{:else}
												<Shield class="size-3" />
												Make admin
											{/if}
										</button>
										<button class="btn btn-xs" onclick={() => revokeSessions(user)}>
											<CircleX class="size-3" />
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
		</fieldset>
	</div>

	<dialog class="modal" bind:this={passwordDialog}>
		<div class="modal-box">
			<h3 class="text-lg font-bold">
				Set password{passwordTarget
					? ` for ${passwordTarget.email === null ? passwordTarget.user_id : passwordTarget.email}`
					: ''}
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

			{#if statusMessage && statusIsError}
				<p class="text-error">{statusMessage}</p>
			{/if}
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>

	<dialog class="modal" bind:this={emailDialog}>
		<div class="modal-box">
			<h3 class="text-lg font-bold">
				Set email address{emailTarget
					? ` for ${emailTarget.email === null ? emailTarget.user_id : emailTarget.email}`
					: ''}
			</h3>

			<label class="label mt-2" for="admin-set-email">New email address</label>
			<input id="admin-set-email" type="email" class="input" bind:value={emailValue} />

			<div class="modal-action">
				<button class="btn btn-ghost" onclick={() => emailDialog.close()}>Cancel</button>
				<button
					class="btn btn-primary"
					disabled={emailValue.length === 0 || emailPending}
					onclick={submitEmail}
					>Set email address
				</button>
			</div>

			{#if statusMessage && statusIsError}
				<p class="text-error">{statusMessage}</p>
			{/if}
		</div>
		<form method="dialog" class="modal-backdrop">
			<button>close</button>
		</form>
	</dialog>
{/if}
