import { graphql } from "$/gql";
import type { Client } from "urql";

export function searchQuery(client: Client, query: string, limit?: number, offset?: number) {
	return client
		.query(
			graphql(`
				query Search($query: String!, $limit: Int, $offset: Int) {
					resp: search(query: $query, limit: $limit, offset: $offset) {
						results {
							object {
								__typename
								... on User {
									id
									username
									displayName
									displayColor {
										color
										hue
										isGray
									}
									channel {
										title
										liveViewerCount
										category {
											name
										}
									}
								}
								... on Category {
									name
								}
							}
							similarity
						}
						totalCount
					}
				}
			`),
			{ query, limit, offset },
			{ requestPolicy: "network-only" },
		)
		.toPromise();
}
