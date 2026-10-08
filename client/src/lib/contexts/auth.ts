import type { AuthInfo } from '#lib/api/index.js';
import type { Option } from '#lib/Options.js';
import { createContext } from 'svelte';

export const [getAuthInfo, setAuthInfo] = createContext<Option<AuthInfo>>();
