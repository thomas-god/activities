import { PUBLIC_APP_URL } from '$env/static/public';

export type AuthInfo = 'NoAuth' | 'SinglePassword' | 'EmailBased';

export const getAuthInfo = async (): Promise<AuthInfo> => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/auth_info`, { method: 'GET' });
	return (await res.text()) as AuthInfo;
};

export const logout = async (): Promise<void> => {
	await fetch(`${PUBLIC_APP_URL}/api/logout`, {
		method: 'POST',
		credentials: 'include',
		mode: 'cors'
	});
};
