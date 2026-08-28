import type { AuthInfo } from '$lib/api';
import type { Option } from '$lib/Options';
import { createContext } from 'svelte';

export const [getAuthInfo, setAuthInfo] = createContext<Option<AuthInfo>>();
