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
