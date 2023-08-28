use std::collections::HashMap;

use super::Ulid;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct S3Bucket {
    pub id: Ulid,
    pub organization_id: Ulid,

    pub name: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub public_url: Option<String>,
    pub managed: bool,

    pub tags: sqlx::types::Json<HashMap<String, String>>,
}
