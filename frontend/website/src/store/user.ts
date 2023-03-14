import { writable } from "svelte/store";

interface User {
	id: number;
	username: string;
	email: string;
	emailVerified: boolean;
	createdAt: string;
	lastLoginAt: string;
}

export const user = writable(null as User | null);
