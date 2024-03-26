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
										rgb
										hsl {
											h
											s
											l
										}
										isGray
									}
									profilePicture {
										id
										variants {
											width
											height
											scale
											url
											format
											byteSize
										}
										endpoint
									}
									channel {
										title
										live {
											liveViewerCount
										}
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
