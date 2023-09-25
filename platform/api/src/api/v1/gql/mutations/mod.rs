use async_graphql::SimpleObject;

mod auth;
mod chat;
mod user;

#[derive(Default, SimpleObject)]
/// The root mutation type which contains root level fields.
pub struct Mutation {
    auth: auth::AuthMutation,
    chat: chat::ChatMutation,
    user: user::UserMutation,
}
