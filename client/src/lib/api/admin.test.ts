import { afterEach, describe, expect, it, vi } from 'vitest';
import {
	AdminApiError,
	createUser,
	listUsers,
	revokeUserSessions,
	setAdmin,
	setUserPassword
} from './admin';

const response = (status: number, body: unknown = null): Response =>
	({
		ok: status >= 200 && status < 300,
		status,
		json: async () => body
	}) as unknown as Response;

describe('admin api', () => {
	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('lists users', async () => {
		const fetchMock = vi.fn().mockResolvedValue(response(200, []));
		vi.stubGlobal('fetch', fetchMock);

		await expect(listUsers()).resolves.toEqual([]);
		expect(fetchMock).toHaveBeenCalledWith(
			expect.stringContaining('/api/admin/users'),
			expect.objectContaining({ method: 'GET' })
		);
	});

	it('throws an AdminApiError carrying the status on failure', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response(403)));

		await expect(listUsers()).rejects.toBeInstanceOf(AdminApiError);
		await expect(listUsers()).rejects.toMatchObject({ status: 403 });
	});

	it('posts a new user', async () => {
		const fetchMock = vi.fn().mockResolvedValue(response(201, { user_id: 'u1' }));
		vi.stubGlobal('fetch', fetchMock);

		await createUser('new@mail.test', 's3cret');

		expect(fetchMock).toHaveBeenCalledWith(
			expect.stringContaining('/api/admin/users'),
			expect.objectContaining({
				method: 'POST',
				body: JSON.stringify({ email: 'new@mail.test', password: 's3cret' })
			})
		);
	});

	it('targets the right user paths for password, admin flag and sessions', async () => {
		const fetchMock = vi.fn().mockResolvedValue(response(200));
		vi.stubGlobal('fetch', fetchMock);

		await setUserPassword('u1', 'pw');
		await setAdmin('u1', true);
		await revokeUserSessions('u1');

		const paths = fetchMock.mock.calls.map(([url]) => url as string);
		expect(paths[0]).toContain('/api/admin/users/u1/password');
		expect(paths[1]).toContain('/api/admin/users/u1/admin');
		expect(paths[2]).toContain('/api/admin/users/u1/sessions/revoke');
	});
});
