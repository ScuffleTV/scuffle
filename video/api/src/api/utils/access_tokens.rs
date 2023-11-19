use std::collections::HashMap;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{AccessTokenScope, Resource};
use tonic::Status;
use video_common::database::AccessToken;

pub struct RequiredScope(Vec<AccessTokenScope>);

pub type ResourcePermission = (Resource, Permission);

impl From<ResourcePermission> for RequiredScope {
	fn from((resource, permission): ResourcePermission) -> Self {
		Self(vec![AccessTokenScope {
			resource: Some(resource.into()),
			permission: vec![permission.into()],
		}])
	}
}

impl From<Vec<ResourcePermission>> for RequiredScope {
	fn from(permissions: Vec<ResourcePermission>) -> Self {
		Self(
			permissions
				.into_iter()
				.map(|(resource, permission)| AccessTokenScope {
					resource: Some(resource.into()),
					permission: vec![permission.into()],
				})
				.collect(),
		)
		.optimize()
	}
}

impl From<Permission> for RequiredScope {
	fn from(permission: Permission) -> Self {
		Self(vec![AccessTokenScope {
			resource: None,
			permission: vec![permission.into()],
		}])
	}
}

impl RequiredScope {
	fn optimize(self) -> Self {
		let mut scopes = self.0;

		scopes.dedup();

		let mut scopes = scopes.into_iter().fold(HashMap::new(), |mut map, new_scope| {
			let resource = new_scope.resource;

			let scope = map.entry(resource).or_insert_with(|| AccessTokenScope {
				resource,
				permission: Vec::new(),
			});

			if scope.permission.contains(&Permission::Admin.into()) {
				return map;
			}

			if new_scope.permission.contains(&Permission::Admin.into()) {
				scope.permission = vec![Permission::Admin.into()];
				return map;
			}

			scope.permission.extend(new_scope.permission);

			scope.permission.sort_unstable();
			scope.permission.dedup();

			map
		});

		if let Some(global_scope) = scopes.remove(&None) {
			if global_scope.permission.contains(&Permission::Admin.into()) {
				return Self(vec![AccessTokenScope {
					resource: None,
					permission: vec![Permission::Admin.into()],
				}]);
			}

			for scope in scopes.values_mut() {
				scope.permission.retain(|p| !global_scope.permission.contains(p));
			}

			scopes.insert(None, global_scope);
		}

		let scopes = scopes.into_values().filter(|s| !s.permission.is_empty()).collect::<Vec<_>>();

		Self(scopes)
	}
}

impl From<AccessTokenScope> for RequiredScope {
	fn from(scope: AccessTokenScope) -> Self {
		Self(vec![scope])
	}
}

impl From<Vec<AccessTokenScope>> for RequiredScope {
	fn from(scopes: Vec<AccessTokenScope>) -> Self {
		Self(scopes)
	}
}

impl std::fmt::Display for RequiredScope {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut permissions = Vec::new();

		for ps in &self.0 {
			let scope = ps
				.resource
				.and_then(|s| Resource::try_from(s).ok())
				.map_or_else(|| "all".to_string(), |r| r.as_str_name().to_lowercase());

			permissions.extend(
				ps.permission
					.iter()
					.filter_map(|p| Permission::try_from(*p).ok())
					.map(|p| format!("{}:{}", scope, p.as_str_name().to_lowercase())),
			);
		}

		permissions.sort();

		permissions.join(" + ").fmt(f)
	}
}

pub trait AccessTokenExt {
	fn has_scope(&self, required: impl Into<RequiredScope>) -> tonic::Result<()>;
}

impl AccessTokenExt for AccessToken {
	fn has_scope(&self, required: impl Into<RequiredScope>) -> tonic::Result<()> {
		let required = required.into().optimize();

		if required.0.iter().all(|required| {
			self.scopes.iter().any(|scope| {
				// Check that the scope is for all resources (unset) or matches the resource in
				// the required scope
				(scope.resource.is_none() || scope.resource == required.resource) &&
                // Check that the scope either has the Admin permission or has all of the required permissions
                (scope.permission.contains(&Permission::Admin.into()) || required.permission.iter().all(|p| scope.permission.contains(p)))
			})
		}) {
			Ok(())
		} else {
			Err(Status::permission_denied(format!("missing required scope: {required}")))
		}
	}
}
