<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
	<circle
		cx="8"
		cy="8"
		r="7"
		stroke="currentColor"
		stroke-width="2"
		stroke-dasharray="44"
		stroke-linecap="round"
	/>
</svg>

<style lang="scss">
	svg {
		animation: spin 0.5s linear infinite;
	}

	circle {
		animation: dash 0.8s linear alternate infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes dash {
		0% {
			stroke-dashoffset: 11;
		}
		100% {
			stroke-dashoffset: 44;
		}
	}
</style>
