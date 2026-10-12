import { PUBLIC_APP_URL } from '$app/env/public';

export type AuthStrategy = 'NoAuth' | 'SinglePassword' | 'MultiUser';

/** Authentication methods enabled on a multi-user instance. */
export interface AuthMethods {
	email: boolean;
	password: boolean;
}

export interface AuthInfo {
	strategy: AuthStrategy;
	/** Whether new users can currently register (only present for the `MultiUser` strategy). */
	registration_open?: boolean;
	/** Authentication methods active on the instance (only present for the `MultiUser` strategy). */
	auth_methods?: AuthMethods;
}

/**
 * Body shared by the multi-user `POST /api/login` and `POST /api/register` requests.
 *
 * `password` is only meaningful when password-based authentication is enabled: it is required to
 * register, and optional to log in — omitting it (or sending an empty string) falls back to the
 * authentication-link email flow when email-based auth is enabled.
 */
export interface MultiUserAuthRequest {
	email: string;
	password?: string;
}

export const fetchAuthInfo = async (): Promise<AuthInfo> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/auth_info`, { method: 'GET' });
	return (await res.json()) as AuthInfo;
};

export const logout = async (): Promise<void> => {
	await fetch(`${PUBLIC_APP_URL}/api/logout`, {
		method: 'POST',
		credentials: 'include',
		mode: 'cors'
	});
};

/** Metadata about the authenticated user's account, from `GET /api/account/me`. */
export interface UserInfo {
	email: string;
	verified: boolean;
	/** Whether the user registered a password: drives set-password vs change-password forms. */
	has_password: boolean;
}

/** Thrown when fetching the user info fails, carrying the HTTP status for control flow. */
export class UserInfoError extends Error {
	constructor(readonly status: number) {
		super(`Failed to fetch user info (status ${status})`);
	}
}

export const fetchUserInfo = async (): Promise<UserInfo> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/account/me`, {
		credentials: 'include',
		mode: 'cors'
	});
	if (!res.ok) {
		throw new UserInfoError(res.status);
	}
	return (await res.json()) as UserInfo;
};

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

/**
 * Requests an email change for the authenticated user.
 *
 * When email-based auth is enabled, a verification link is sent to the new address and the change
 * only takes effect once it is validated (`validateEmailChange`): the response is `202 Accepted`.
 * When email-based auth is disabled, the change is applied synchronously (marked unverified) and
 * the response is `200 OK`. Both are answered identically when the target email is already used
 * by another account, so the response never leaks account existence.
 */
export const requestEmailChange = (newEmail: string): Promise<Response> =>
	postJson('/api/account/email', { new_email: newEmail });

/**
 * Validates an email change verification link. Unauthenticated on purpose: the link may be
 * opened from a different device than the one that requested the change.
 *
 * Statuses: `200 OK` (the change is applied, body carries the new email), `409 Conflict` (the
 * target email is now used by another account — reported explicitly here, unlike the request
 * step, because a valid token proves control of the address), `401 Unauthorized` (invalid or
 * expired token), `503 Service Unavailable`.
 */
export const validateEmailChange = (token: string): Promise<Response> =>
	postJson('/api/email/change/validate', { token });

/** Registers the first password for the authenticated user. `409 Conflict` if one is already set. */
export const registerPassword = (newPassword: string): Promise<Response> =>
	postJson('/api/password', { new_password: newPassword });

/**
 * Replaces the user's password, requiring the current one. `401 Unauthorized` covers both an
 * unknown account and a wrong current password (indistinguishable on purpose).
 */
export const changePassword = (currentPassword: string, newPassword: string): Promise<Response> =>
	postJson('/api/password/change', {
		current_password: currentPassword,
		new_password: newPassword
	});
