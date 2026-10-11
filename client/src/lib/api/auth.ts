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
