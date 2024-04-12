import type { User } from "$/gql/graphql";
import { writable } from "svelte/store";

export enum AuthMode {
	Login,
	Register,
}

export type AuthDialog = {
	opened: boolean;
	mode: AuthMode;
};

export const authDialog = writable<AuthDialog>({
	opened: false,
	mode: AuthMode.Login,
});
export const currentTwoFaRequest = writable<string | null>(null);
export const sessionToken = writable<string | null>(undefined);
export const user = writable<User | null>(null);

// This is a separate store for the user id so that we can only subscribe to id changes instead of the whole user store
export const userId = writable<string | null>(null);
user.subscribe((user) => {
	userId.set(user?.id ?? null);
});
