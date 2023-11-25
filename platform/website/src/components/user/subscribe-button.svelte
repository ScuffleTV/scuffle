<script lang="ts">
	import Fa from "svelte-fa";
	import { faFireFlameCurved } from "@fortawesome/free-solid-svg-icons";
	import { AuthMode, authDialog, user } from "$/store/auth";

	function onClick() {
		if ($user) {
			console.log("subscribe");
		} else {
			$authDialog = {
				opened: true,
				mode: AuthMode.Login,
			};
		}
	}
</script>

<button class="button primary" on:click={onClick}>
	<Fa icon={faFireFlameCurved} />
	<span>Subscribe</span>
</button>

<style lang="scss">
	.button {
		padding: 0.4rem 0.8rem;
		font-weight: 500;

		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
</style>
