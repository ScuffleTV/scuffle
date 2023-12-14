import { redirect } from "@sveltejs/kit";
import { AuthMode, authDialog } from "$/store/auth";

export async function load() {
	authDialog.set({
		opened: true,
		mode: AuthMode.Register,
	});
	throw redirect(301, "/");
}
