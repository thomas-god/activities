import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';
import { AdminApiError, listUsers, type AdminUser } from '#lib/api/index.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async (): Promise<{
	users: AdminUser[] | null;
	forbidden: boolean;
}> => {
	try {
		return { users: await listUsers(), forbidden: false };
	} catch (error) {
		if (error instanceof AdminApiError && error.status === 401) {
			// No session (or expired): the page is only meaningful when authenticated.
			redirect(302, resolve('/login'));
		}
		if (error instanceof AdminApiError && error.status === 403) {
			return { users: null, forbidden: true };
		}
		// Server unavailable etc.: render the page so it can offer a retry.
		return { users: null, forbidden: false };
	}
};
