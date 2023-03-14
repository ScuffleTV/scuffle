import { writable } from "svelte/store";

interface Token {
	token: string;
	source: "localstorage" | "auth";
}

export const loginMode = writable(0);
export const sessionToken = writable(null as Token | null);

// When we mount we need to check if we have a token in local storage
// and if so, set it in the store
if (typeof window !== "undefined") {
	const localToken = localStorage.getItem("SCUFFLE_SESSION_TOKEN");
	if (localToken) {
		sessionToken.set({
			token: localToken,
			source: "localstorage",
		});
	}

	sessionToken.subscribe(async (token) => {
		if (token) {
			localStorage.setItem("SCUFFLE_SESSION_TOKEN", token.token);
		} else {
			localStorage.removeItem("SCUFFLE_SESSION_TOKEN");
		}
	});
}
