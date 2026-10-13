import type { AuthInfo, UserInfo } from '#lib/api/index.js';
import type { Option } from '#lib/Options.js';
import { createContext } from 'svelte';

export const [getAuthInfo, setAuthInfo] = createContext<Option<AuthInfo>>();

/** The authenticated user, fetched once in the root layout for multi-user instances. */
export const [getCurrentUser, setCurrentUser] = createContext<Option<UserInfo>>();
