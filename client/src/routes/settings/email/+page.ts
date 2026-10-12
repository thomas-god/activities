import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';
import { fetchUserInfo, UserInfoError, type UserInfo } from '#lib/api/index.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async (): Promise<{ userInfo: UserInfo | null }> => {
	try {
		return { userInfo: await fetchUserInfo() };
	} catch (error) {
		if (error instanceof UserInfoError && error.status === 401) {
			// No session (or expired): the settings page is only meaningful when authenticated.
			redirect(302, resolve('/login'));
		}
		// Other failures (server unavailable, etc.): render the page so it can offer a retry
		// instead of bouncing the user to the login form.
		return { userInfo: null };
	}
};
