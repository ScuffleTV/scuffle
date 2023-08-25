<script>
	import { user } from "$/store/auth";
	import DefaultAvatar from "../user/default-avatar.svelte";
	import DisplayName from "../user/display-name.svelte";
</script>

{#if $user}
	<div class="user-banner">
		<DefaultAvatar userId={$user.id} bind:displayColor={$user.displayColor} size={6 * 16} />
		<div class="user-info">
			<span class="display-name">
				<DisplayName userId={$user.id} bind:displayName={$user.displayName} />
			</span>
			<span class="username">{$user.username}</span>
		</div>
	</div>
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.user-banner {
		padding: 1rem 2rem;
		border: $borderColor solid 1px;
		border-radius: 0.5rem;
		background: linear-gradient(135deg, rgba($primaryColor, 0.05), rgba(white, 0.05));

		display: flex;
		align-items: center;
		gap: 1rem;

		& > .user-info {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;

			& > .display-name {
				font-weight: 500;
				font-size: 1.75rem;
				line-height: 0.8;
			}

			& > .username {
				line-height: 0.8;
				font-weight: 500;
				font-size: 1.25rem;
				color: $textColorLight;
			}
		}
	}
</style>
