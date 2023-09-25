use async_graphql::SimpleObject;

use crate::database::{self, SearchResult};

use super::date::DateRFC3339;
use super::ulid::GqlUlid;

#[derive(SimpleObject, Clone)]
pub struct CategorySearchResult {
    category: Category,
    similarity: f64,
}

impl From<SearchResult<database::Category>> for CategorySearchResult {
    fn from(value: SearchResult<database::Category>) -> Self {
        Self {
            category: value.object.into(),
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

impl From<database::Category> for Category {
    fn from(value: database::Category) -> Self {
        Self {
            id: value.id.0.into(),
            name: value.name,
            revision: value.revision,
            updated_at: value.updated_at.into(),
        }
    }
}
