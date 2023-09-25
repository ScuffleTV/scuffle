import type { User } from "$/gql/graphql";
import type { verifyToken } from "$/lib/auth";
import { writable } from "svelte/store";

export enum AuthDialog {
	Closed,
	Login,
	Register,
	SolveTwoFa,
}

type Session = Awaited<ReturnType<typeof verifyToken>>;

export const authDialog = writable(AuthDialog.Closed);
export const session = writable<Session | null>(undefined);
export const user = writable<User | null>(null);

session.subscribe((data) => {
	if (data && !data.twoFaSolved) {
		authDialog.set(AuthDialog.SolveTwoFa);
	}
});
