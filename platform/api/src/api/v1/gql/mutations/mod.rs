use async_graphql::SimpleObject;

mod auth;
mod chat;
mod two_fa;
mod user;

#[derive(Default, SimpleObject)]
/// The root mutation type which contains root level fields.
pub struct Mutation {
    auth: auth::AuthMutation,
    chat: chat::ChatMutation,
    two_fa: two_fa::TwoFaMutation,
    user: user::UserMutation,
}
