import type { User } from "$/gql/graphql";
import { browser } from "$app/environment";
import { writable } from "svelte/store";

function sessionTokenInit() {
	if (browser) {
		return window.localStorage.getItem("auth_sessionToken") ?? null;
	}
	return null;
}

export enum AuthDialog {
	Closed,
	Login,
	Register,
}

export const authDialog = writable(AuthDialog.Closed);
export const sessionToken = writable<string | null>(sessionTokenInit());
export const user = writable<User | null>(null);

// Save session token to cookies when changed
if (browser) {
	sessionToken.subscribe((token) => {
		if (token) {
			localStorage.setItem("auth_sessionToken", token);
		} else {
			localStorage.removeItem("auth_sessionToken");
		}
	});
}
