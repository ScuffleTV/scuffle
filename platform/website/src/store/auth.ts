import type { User } from "$/gql/graphql";
import { writable } from "svelte/store";

export enum AuthDialog {
	Closed,
	Login,
	Register,
}

export const authDialog = writable(AuthDialog.Closed);
export const currentTwoFaRequest = writable<string | null>(null);
export const sessionToken = writable<string | null>(undefined);
export const user = writable<User | null>(null);
