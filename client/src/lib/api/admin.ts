import { PUBLIC_APP_URL } from '$app/env/public';

/** A user as returned by the admin user-management API (`GET /api/admin/users`). */
export interface AdminUser {
	user_id: string;
	email: string | null;
	/** Whether the stored email still parses as an email address. */
	email_valid: boolean;
	verified: boolean;
	has_password: boolean;
	is_admin: boolean;
}

/** Thrown by admin calls that fail before the caller can branch on the HTTP status. */
export class AdminApiError extends Error {
	constructor(readonly status: number) {
		super(`Admin request failed (status ${status})`);
	}
}

const adminFetch = (path: string, init: RequestInit): Promise<Response> =>
	fetch(`${PUBLIC_APP_URL}${path}`, { credentials: 'include', mode: 'cors', ...init });

const postJson = (path: string, body: object): Promise<Response> =>
	adminFetch(path, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});

/** Lists every user. Throws {@link AdminApiError} with the status on failure (`401`, `403`, …). */
export const listUsers = async (): Promise<AdminUser[]> => {
	const res = await adminFetch('/api/admin/users', { method: 'GET' });
	if (!res.ok) {
		throw new AdminApiError(res.status);
	}
	return (await res.json()) as AdminUser[];
};

/** Creates a user even when registration is closed. `409` when the email is already used. */
export const createUser = (email: string, password: string): Promise<Response> =>
	postJson('/api/admin/users', { email, password });

/** Sets or resets a user's password without the current one. `404` when the user is unknown. */
export const setUserPassword = (userId: string, password: string): Promise<Response> =>
	postJson(`/api/admin/users/${encodeURIComponent(userId)}/password`, { password });

/** Grants or revokes the admin flag. `409` when it would remove the last admin. */
export const setAdmin = (userId: string, isAdmin: boolean): Promise<Response> =>
	postJson(`/api/admin/users/${encodeURIComponent(userId)}/admin`, { is_admin: isAdmin });

/** Revokes every session of the user, returning how many were removed. */
export const revokeUserSessions = (userId: string): Promise<Response> =>
	postJson(`/api/admin/users/${encodeURIComponent(userId)}/sessions/revoke`, {});
