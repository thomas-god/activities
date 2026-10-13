import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';
import {
	AdminApiError,
	fetchUserInfo,
	listUsers,
	UserInfoError,
	type AdminUser,
	type UserInfo
} from '#lib/api/index.js';
import type { PageLoad } from './$types';
import { none, some, type Option } from '#lib/Options.ts';

export type AdminInfo = { users: AdminUser[] };

export const load: PageLoad = async (): Promise<{
	userInfo: UserInfo | null;
	adminInfo: Option<AdminInfo>;
}> => {
	try {
		return { userInfo: await fetchUserInfo(), adminInfo: await getAdminInfo() };
	} catch (error) {
		if (error instanceof UserInfoError && error.status === 401) {
			// No session (or expired): the settings page is only meaningful when authenticated.
			redirect(302, resolve('/login'));
		}
		// Other failures (server unavailable, etc.): render the page so it can offer a retry
		// instead of bouncing the user to the login form.
		return { userInfo: null, adminInfo: none() };
	}
};

const getAdminInfo = async (): Promise<Option<AdminInfo>> => {
	try {
		return some({ users: await listUsers() });
	} catch (error) {
		if (error instanceof AdminApiError && error.status === 401) {
			// No session (or expired): the page is only meaningful when authenticated.
			redirect(302, resolve('/login'));
		}
		if (error instanceof AdminApiError && error.status === 403) {
			return none();
		}
		// Server unavailable etc.: render the page so it can offer a retry.
		return none();
	}
};
