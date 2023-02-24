import { writable } from "svelte/store";

export const loginMode = writable(0);
export const sessionToken = writable("");

// When we mount we need to check if we have a token in local storage
// and if so, set it in the store
if (typeof window !== "undefined") {
	const localToken = localStorage.getItem("SCUFFLE_SESSION_TOKEN");
	if (localToken) {
		sessionToken.set(localToken);
	}

	sessionToken.subscribe((token) => {
		if (token) {
			localStorage.setItem("SCUFFLE_SESSION_TOKEN", token);
		} else {
			localStorage.removeItem("SCUFFLE_SESSION_TOKEN");
		}
	});
}
