// The magic-link token arrives as a URL fragment (never sent to the server), so this page
// must always be rendered client-side to read it from `window.location.hash`.
export const prerender = false;
export const ssr = false;
