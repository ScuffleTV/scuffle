<div class="spinner">
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="rect"></span>
	<span class="alt">Loading...</span>
</div>

<style lang="scss">
	.spinner {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		grid-template-rows: repeat(3, 1fr);
		gap: 0.25rem;

		@media (prefers-reduced-motion: reduce) {
			& > .rect {
				display: none;
			}
		}

		@media not (prefers-reduced-motion: reduce) {
			.alt {
				display: none;
			}

			.rect:nth-child(5) {
				opacity: 0;
			}

			$speed: 0.1s;
			$order: (1, 2, 3, 6, 9, 8, 7, 4);

			@for $i from 1 through length($order) {
				$child: nth($order, $i);

				.rect:nth-child(#{$child}) {
					opacity: 0;
					animation: spin calc(length($order) * $speed) infinite linear;
					animation-delay: calc($i * $speed);
				}
			}

			@keyframes spin {
				from {
					opacity: 1;
				}
				50% {
					opacity: 0;
				}
				to {
					opacity: 0;
				}
			}
		}

		.alt {
			grid-row: 1 / -1;
			grid-column: 1 / -1;
		}

		.rect {
			text-align: center;
			width: 1rem;
			height: 1rem;
			background-color: white;
		}
	}
</style>
