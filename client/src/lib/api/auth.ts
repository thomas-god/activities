import { PUBLIC_APP_URL } from '$env/static/public';

export type AuthStrategy = 'NoAuth' | 'SinglePassword' | 'EmailBased';

export interface AuthInfo {
	strategy: AuthStrategy;
	/** Whether new users can currently register (only relevant in EmailBased mode). */
	registration: boolean;
}

export const getAuthInfo = async (): Promise<AuthInfo> => {
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
