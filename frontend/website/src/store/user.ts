import { writable } from "svelte/store";

import type { User } from "../types/index";

export const user = writable(null as User | null);
