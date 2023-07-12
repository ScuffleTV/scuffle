import { writable } from "svelte/store";
import { browser } from "$app/environment";

export const sideNavCollapsed = writable(
	browser && localStorage.getItem("layout_sideNavCollapsed") === "true",
);

export const sideNavHidden = writable(false);

export const topNavHidden = writable(false);

sideNavCollapsed.subscribe((value) => {
	if (browser) {
		localStorage.setItem("layout_sideNavCollapsed", JSON.stringify(value));
	}
});
