use async_graphql::SimpleObject;

use crate::database::category;

use super::date::DateRFC3339;
use super::ulid::GqlUlid;

#[derive(SimpleObject, Clone)]
pub struct CategorySearchResult {
    category: Category,
    similarity: f64,
}

impl From<category::SearchResult> for CategorySearchResult {
    fn from(value: category::SearchResult) -> Self {
        Self {
            category: value.category.into(),
            similarity: value.similarity,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Category {
    pub id: GqlUlid,
    pub name: String,
    pub revision: i32,
    pub updated_at: DateRFC3339,
}

impl From<category::Model> for Category {
    fn from(value: category::Model) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
            revision: value.revision,
            updated_at: value.updated_at.into(),
        }
    }
}
