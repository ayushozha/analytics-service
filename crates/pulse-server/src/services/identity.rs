use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub project_id: Uuid,
    pub visitor_id: String,
    pub user_id: Option<String>,
    pub traits: serde_json::Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub identified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserAlias {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: String,
    pub visitor_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AccountProfile {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: String,
    pub name: Option<String>,
    pub traits: serde_json::Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AccountMembership {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: String,
    pub user_id: Option<String>,
    pub visitor_id: String,
    pub role: Option<String>,
    pub traits: serde_json::Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountAnalytics {
    pub account_id: String,
    pub name: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub members: i64,
    pub identified_users: i64,
    pub sessions: i64,
    pub pageviews: i64,
    pub events: i64,
    pub revenue: f64,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScimUser {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_name: String,
    pub external_id: Option<String>,
    pub active: bool,
    pub display_name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub emails: serde_json::Value,
    pub traits: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScimGroup {
    pub id: Uuid,
    pub project_id: Uuid,
    pub display_name: String,
    pub external_id: Option<String>,
    pub traits: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimGroupWithMembers {
    pub group: ScimGroup,
    pub members: Vec<ScimUser>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScimUserInput {
    pub user_name: String,
    pub external_id: Option<String>,
    #[serde(default = "default_active")]
    pub active: bool,
    pub display_name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    #[serde(default = "empty_array")]
    pub emails: serde_json::Value,
    #[serde(default = "empty_object")]
    pub traits: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScimGroupInput {
    pub display_name: String,
    pub external_id: Option<String>,
    #[serde(default = "empty_object")]
    pub traits: serde_json::Value,
    #[serde(default)]
    pub members: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IdentityGraphNode {
    pub id: String,
    pub node_type: String,
    pub key: String,
    pub label: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IdentityGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IdentityGraph {
    pub nodes: Vec<IdentityGraphNode>,
    pub edges: Vec<IdentityGraphEdge>,
}

const USER_PROFILE_COLUMNS: &str = "id, project_id, visitor_id, user_id, traits, first_seen_at, \
    last_seen_at, identified_at, created_at, updated_at";
const ACCOUNT_PROFILE_COLUMNS: &str = "id, project_id, account_id, name, traits, first_seen_at, \
    last_seen_at, created_at, updated_at";
const ACCOUNT_MEMBERSHIP_COLUMNS: &str = "id, project_id, account_id, user_id, visitor_id, role, \
    traits, first_seen_at, last_seen_at, created_at, updated_at";
const SCIM_USER_COLUMNS: &str = "id, project_id, user_name, external_id, active, display_name, \
    given_name, family_name, emails, traits, created_at, updated_at";
const SCIM_USER_JOIN_COLUMNS: &str = "u.id, u.project_id, u.user_name, u.external_id, u.active, \
    u.display_name, u.given_name, u.family_name, u.emails, u.traits, u.created_at, u.updated_at";
const SCIM_GROUP_COLUMNS: &str =
    "id, project_id, display_name, external_id, traits, created_at, updated_at";

fn default_active() -> bool {
    true
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn empty_array() -> serde_json::Value {
    serde_json::json!([])
}

/// Persist an identify call and merge traits into the visitor's profile.
pub async fn identify_user(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    user_id: Option<&str>,
    traits: &serde_json::Value,
    account_id: Option<&str>,
    account_name: Option<&str>,
    account_traits: Option<&serde_json::Value>,
    account_role: Option<&str>,
    identified_at: DateTime<Utc>,
) -> AppResult<UserProfile> {
    let user_id = normalize_optional(user_id);
    let profile: UserProfile = sqlx::query_as(&format!(
        "INSERT INTO user_profiles \
         (project_id, visitor_id, user_id, traits, first_seen_at, last_seen_at, identified_at) \
         VALUES ($1, $2, $3, $4, $5, $5, $5) \
         ON CONFLICT (project_id, visitor_id) DO UPDATE SET \
             user_id = COALESCE(EXCLUDED.user_id, user_profiles.user_id), \
             traits = user_profiles.traits || EXCLUDED.traits, \
             first_seen_at = LEAST(user_profiles.first_seen_at, EXCLUDED.first_seen_at), \
             last_seen_at = GREATEST(user_profiles.last_seen_at, EXCLUDED.last_seen_at), \
             identified_at = COALESCE(user_profiles.identified_at, EXCLUDED.identified_at), \
             updated_at = NOW() \
         RETURNING {USER_PROFILE_COLUMNS}"
    ))
    .bind(project_id)
    .bind(visitor_id)
    .bind(user_id)
    .bind(traits)
    .bind(identified_at)
    .fetch_one(db)
    .await?;

    if let Some(user_id) = user_id {
        sqlx::query(
            "INSERT INTO user_aliases (project_id, user_id, visitor_id) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (project_id, user_id, visitor_id) DO NOTHING",
        )
        .bind(project_id)
        .bind(user_id)
        .bind(visitor_id)
        .execute(db)
        .await?;
    }

    if let Some(account_id) = normalize_optional(account_id) {
        let account_name = normalize_optional(account_name);
        let account_role = normalize_optional(account_role);
        let account_traits = normalized_json_object(account_traits, "account_traits")?;

        upsert_account(
            db,
            project_id,
            account_id,
            account_name,
            &account_traits,
            identified_at,
        )
        .await?;
        upsert_account_membership(
            db,
            project_id,
            account_id,
            user_id,
            visitor_id,
            account_role,
            identified_at,
        )
        .await?;
    }

    Ok(profile)
}

pub async fn list_profiles(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<UserProfile>> {
    let profiles: Vec<UserProfile> = sqlx::query_as(&format!(
        "SELECT {USER_PROFILE_COLUMNS} FROM user_profiles \
         WHERE project_id = $1 \
         ORDER BY updated_at DESC \
         LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(profiles)
}

pub async fn get_profile_by_visitor(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
) -> AppResult<UserProfile> {
    let profile: Option<UserProfile> = sqlx::query_as(&format!(
        "SELECT {USER_PROFILE_COLUMNS} FROM user_profiles \
         WHERE project_id = $1 AND visitor_id = $2"
    ))
    .bind(project_id)
    .bind(visitor_id)
    .fetch_optional(db)
    .await?;

    profile.ok_or_else(|| AppError::NotFound("User profile not found".to_string()))
}

pub async fn list_aliases_for_user(
    db: &PgPool,
    project_id: Uuid,
    user_id: &str,
) -> AppResult<Vec<UserAlias>> {
    let aliases: Vec<UserAlias> = sqlx::query_as(
        "SELECT id, project_id, user_id, visitor_id, created_at \
         FROM user_aliases \
         WHERE project_id = $1 AND user_id = $2 \
         ORDER BY created_at DESC",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(aliases)
}

pub async fn list_accounts(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AccountProfile>> {
    let accounts = sqlx::query_as(&format!(
        "SELECT {ACCOUNT_PROFILE_COLUMNS} FROM account_profiles \
         WHERE project_id = $1 \
         ORDER BY last_seen_at DESC \
         LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;

    Ok(accounts)
}

pub async fn get_account(
    db: &PgPool,
    project_id: Uuid,
    account_id: &str,
) -> AppResult<AccountProfile> {
    let account = sqlx::query_as(&format!(
        "SELECT {ACCOUNT_PROFILE_COLUMNS} FROM account_profiles \
         WHERE project_id = $1 AND account_id = $2"
    ))
    .bind(project_id)
    .bind(account_id)
    .fetch_optional(db)
    .await?;

    account.ok_or_else(|| AppError::NotFound("Account not found".to_string()))
}

pub async fn list_account_members(
    db: &PgPool,
    project_id: Uuid,
    account_id: &str,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AccountMembership>> {
    let members = sqlx::query_as(&format!(
        "SELECT {ACCOUNT_MEMBERSHIP_COLUMNS} FROM account_memberships \
         WHERE project_id = $1 AND account_id = $2 \
         ORDER BY last_seen_at DESC \
         LIMIT $3 OFFSET $4"
    ))
    .bind(project_id)
    .bind(account_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;

    Ok(members)
}

pub async fn get_account_analytics(
    db: &PgPool,
    project_id: Uuid,
    account_id: &str,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<AccountAnalytics> {
    let account = get_account(db, project_id, account_id).await?;
    let (members, identified_users, sessions, pageviews, events, revenue): (
        i64,
        i64,
        i64,
        i64,
        i64,
        f64,
    ) = sqlx::query_as(
        "WITH members AS ( \
           SELECT visitor_id, user_id FROM account_memberships \
           WHERE project_id = $1 AND account_id = $2 \
         ) \
         SELECT \
           (SELECT COUNT(*)::bigint FROM members) AS members, \
           (SELECT COUNT(DISTINCT user_id)::bigint FROM members WHERE user_id IS NOT NULL) AS identified_users, \
           (SELECT COUNT(DISTINCT s.id)::bigint FROM sessions s JOIN members m ON m.visitor_id = s.visitor_id \
             WHERE s.project_id = $1 AND s.first_at >= $3 AND s.first_at <= $4) AS sessions, \
           (SELECT COUNT(*)::bigint FROM pageviews p JOIN members m ON m.visitor_id = p.visitor_id \
             WHERE p.project_id = $1 AND p.created_at >= $3 AND p.created_at <= $4) AS pageviews, \
           (SELECT COUNT(*)::bigint FROM events e JOIN members m ON m.visitor_id = e.visitor_id \
             WHERE e.project_id = $1 AND e.created_at >= $3 AND e.created_at <= $4) AS events, \
           (SELECT COALESCE(SUM(e.revenue_amount), 0)::float8 FROM events e JOIN members m ON m.visitor_id = e.visitor_id \
             WHERE e.project_id = $1 AND e.created_at >= $3 AND e.created_at <= $4) AS revenue",
    )
    .bind(project_id)
    .bind(account_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_one(db)
    .await?;

    Ok(AccountAnalytics {
        account_id: account.account_id,
        name: account.name,
        start_at,
        end_at,
        members,
        identified_users,
        sessions,
        pageviews,
        events,
        revenue,
        last_seen_at: account.last_seen_at,
    })
}

pub async fn list_scim_users(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<ScimUser>> {
    let users = sqlx::query_as(&format!(
        "SELECT {SCIM_USER_COLUMNS} FROM scim_users \
         WHERE project_id = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(users)
}

pub async fn get_scim_user(db: &PgPool, project_id: Uuid, user_id: Uuid) -> AppResult<ScimUser> {
    sqlx::query_as(&format!(
        "SELECT {SCIM_USER_COLUMNS} FROM scim_users WHERE id = $1 AND project_id = $2"
    ))
    .bind(user_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("SCIM user not found".to_string()))
}

pub async fn create_scim_user(
    db: &PgPool,
    project_id: Uuid,
    input: ScimUserInput,
) -> AppResult<ScimUser> {
    let input = validate_scim_user_input(input)?;
    let user: ScimUser = sqlx::query_as(&format!(
        "INSERT INTO scim_users \
         (project_id, user_name, external_id, active, display_name, given_name, family_name, emails, traits) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING {SCIM_USER_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.user_name)
    .bind(&input.external_id)
    .bind(input.active)
    .bind(&input.display_name)
    .bind(&input.given_name)
    .bind(&input.family_name)
    .bind(&input.emails)
    .bind(&input.traits)
    .fetch_one(db)
    .await?;
    sync_scim_user_to_identity(db, &user).await?;
    Ok(user)
}

pub async fn update_scim_user(
    db: &PgPool,
    project_id: Uuid,
    user_id: Uuid,
    input: ScimUserInput,
) -> AppResult<ScimUser> {
    let input = validate_scim_user_input(input)?;
    let user: ScimUser = sqlx::query_as(&format!(
        "UPDATE scim_users SET \
           user_name = $3, external_id = $4, active = $5, display_name = $6, given_name = $7, \
           family_name = $8, emails = $9, traits = $10, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 RETURNING {SCIM_USER_COLUMNS}"
    ))
    .bind(user_id)
    .bind(project_id)
    .bind(&input.user_name)
    .bind(&input.external_id)
    .bind(input.active)
    .bind(&input.display_name)
    .bind(&input.given_name)
    .bind(&input.family_name)
    .bind(&input.emails)
    .bind(&input.traits)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("SCIM user not found".to_string()))?;
    sync_scim_user_to_identity(db, &user).await?;
    Ok(user)
}

pub async fn delete_scim_user(db: &PgPool, project_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM scim_users WHERE id = $1 AND project_id = $2")
        .bind(user_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("SCIM user not found".to_string()));
    }
    Ok(())
}

pub async fn list_scim_groups(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<ScimGroup>> {
    let groups = sqlx::query_as(&format!(
        "SELECT {SCIM_GROUP_COLUMNS} FROM scim_groups \
         WHERE project_id = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(groups)
}

pub async fn get_scim_group(
    db: &PgPool,
    project_id: Uuid,
    group_id: Uuid,
) -> AppResult<ScimGroupWithMembers> {
    let group: ScimGroup = sqlx::query_as(&format!(
        "SELECT {SCIM_GROUP_COLUMNS} FROM scim_groups WHERE id = $1 AND project_id = $2"
    ))
    .bind(group_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("SCIM group not found".to_string()))?;
    let members = scim_group_members(db, project_id, group_id).await?;
    Ok(ScimGroupWithMembers { group, members })
}

pub async fn create_scim_group(
    db: &PgPool,
    project_id: Uuid,
    input: ScimGroupInput,
) -> AppResult<ScimGroupWithMembers> {
    let input = validate_scim_group_input(input)?;
    ensure_scim_users_exist(db, project_id, &input.members).await?;
    let group: ScimGroup = sqlx::query_as(&format!(
        "INSERT INTO scim_groups (project_id, display_name, external_id, traits) \
         VALUES ($1, $2, $3, $4) RETURNING {SCIM_GROUP_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.display_name)
    .bind(&input.external_id)
    .bind(&input.traits)
    .fetch_one(db)
    .await?;
    replace_scim_group_members(db, project_id, group.id, &input.members).await?;
    get_scim_group(db, project_id, group.id).await
}

pub async fn update_scim_group(
    db: &PgPool,
    project_id: Uuid,
    group_id: Uuid,
    input: ScimGroupInput,
) -> AppResult<ScimGroupWithMembers> {
    let input = validate_scim_group_input(input)?;
    ensure_scim_users_exist(db, project_id, &input.members).await?;
    let group: ScimGroup = sqlx::query_as(&format!(
        "UPDATE scim_groups SET display_name = $3, external_id = $4, traits = $5, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 RETURNING {SCIM_GROUP_COLUMNS}"
    ))
    .bind(group_id)
    .bind(project_id)
    .bind(&input.display_name)
    .bind(&input.external_id)
    .bind(&input.traits)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("SCIM group not found".to_string()))?;
    replace_scim_group_members(db, project_id, group.id, &input.members).await?;
    get_scim_group(db, project_id, group.id).await
}

pub async fn delete_scim_group(db: &PgPool, project_id: Uuid, group_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM scim_groups WHERE id = $1 AND project_id = $2")
        .bind(group_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("SCIM group not found".to_string()));
    }
    Ok(())
}

pub async fn get_identity_graph(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: Option<&str>,
    user_id: Option<&str>,
    account_id: Option<&str>,
    limit: i64,
) -> AppResult<IdentityGraph> {
    let visitor_id = normalize_optional(visitor_id);
    let user_id = normalize_optional(user_id);
    let account_id = normalize_optional(account_id);
    if visitor_id.is_none() && user_id.is_none() && account_id.is_none() {
        return Err(AppError::BadRequest(
            "Provide visitor_id, user_id, or account_id".to_string(),
        ));
    }

    let limit = limit.clamp(1, 500);
    let mut builder = IdentityGraphBuilder::default();
    if let Some(visitor_id) = visitor_id {
        builder.add_visitor(visitor_id, serde_json::json!({ "seed": true }));
    }
    if let Some(user_id) = user_id {
        builder.add_user(user_id, serde_json::json!({ "seed": true }));
    }
    if let Some(account_id) = account_id {
        builder.add_account(account_id, None, serde_json::json!({ "seed": true }));
    }

    let visitor_ids =
        related_visitor_ids(db, project_id, visitor_id, user_id, account_id, limit).await?;
    let profiles = graph_profiles(db, project_id, &visitor_ids, user_id, limit).await?;
    let aliases = graph_aliases(db, project_id, &visitor_ids, user_id, limit).await?;
    let memberships =
        graph_memberships(db, project_id, &visitor_ids, user_id, account_id, limit).await?;

    let mut account_ids = BTreeSet::new();
    if let Some(account_id) = account_id {
        account_ids.insert(account_id.to_string());
    }
    for membership in &memberships {
        account_ids.insert(membership.account_id.clone());
    }
    let accounts = graph_accounts(db, project_id, &account_ids).await?;

    for profile in &profiles {
        builder.add_visitor(
            &profile.visitor_id,
            serde_json::json!({
                "traits": profile.traits,
                "first_seen_at": profile.first_seen_at,
                "last_seen_at": profile.last_seen_at,
                "identified_at": profile.identified_at,
            }),
        );
        if let Some(user_id) = &profile.user_id {
            builder.add_user(user_id, serde_json::json!({}));
            builder.add_edge(
                &node_id("visitor", &profile.visitor_id),
                &node_id("user", user_id),
                "identified_as",
                serde_json::json!({
                    "identified_at": profile.identified_at,
                }),
            );
        }
    }

    for alias in &aliases {
        builder.add_visitor(&alias.visitor_id, serde_json::json!({}));
        builder.add_user(&alias.user_id, serde_json::json!({}));
        builder.add_edge(
            &node_id("visitor", &alias.visitor_id),
            &node_id("user", &alias.user_id),
            "alias",
            serde_json::json!({ "created_at": alias.created_at }),
        );
    }

    for account in &accounts {
        builder.add_account(
            &account.account_id,
            account.name.as_deref(),
            serde_json::json!({
                "traits": account.traits,
                "first_seen_at": account.first_seen_at,
                "last_seen_at": account.last_seen_at,
            }),
        );
    }

    for membership in &memberships {
        builder.add_visitor(&membership.visitor_id, serde_json::json!({}));
        builder.add_account(&membership.account_id, None, serde_json::json!({}));
        builder.add_edge(
            &node_id("visitor", &membership.visitor_id),
            &node_id("account", &membership.account_id),
            "member_of",
            serde_json::json!({
                "role": membership.role,
                "traits": membership.traits,
                "first_seen_at": membership.first_seen_at,
                "last_seen_at": membership.last_seen_at,
            }),
        );

        if let Some(user_id) = &membership.user_id {
            builder.add_user(user_id, serde_json::json!({}));
            builder.add_edge(
                &node_id("user", user_id),
                &node_id("account", &membership.account_id),
                "member_of",
                serde_json::json!({
                    "role": membership.role,
                    "last_seen_at": membership.last_seen_at,
                }),
            );
            builder.add_edge(
                &node_id("visitor", &membership.visitor_id),
                &node_id("user", user_id),
                "identified_as",
                serde_json::json!({ "source": "account_membership" }),
            );
        }
    }

    Ok(builder.finish())
}

async fn related_visitor_ids(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: Option<&str>,
    user_id: Option<&str>,
    account_id: Option<&str>,
    limit: i64,
) -> AppResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT visitor_id FROM ( \
           SELECT visitor_id FROM user_profiles \
           WHERE project_id = $1 AND (($2::text IS NOT NULL AND visitor_id = $2) OR ($3::text IS NOT NULL AND user_id = $3)) \
           UNION \
           SELECT visitor_id FROM user_aliases \
           WHERE project_id = $1 AND (($2::text IS NOT NULL AND visitor_id = $2) OR ($3::text IS NOT NULL AND user_id = $3)) \
           UNION \
           SELECT visitor_id FROM account_memberships \
           WHERE project_id = $1 AND (($2::text IS NOT NULL AND visitor_id = $2) OR ($3::text IS NOT NULL AND user_id = $3) OR ($4::text IS NOT NULL AND account_id = $4)) \
         ) related \
         ORDER BY visitor_id \
         LIMIT $5",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(user_id)
    .bind(account_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(visitor_id,)| visitor_id).collect())
}

async fn graph_profiles(
    db: &PgPool,
    project_id: Uuid,
    visitor_ids: &[String],
    user_id: Option<&str>,
    limit: i64,
) -> AppResult<Vec<UserProfile>> {
    let profiles = sqlx::query_as(&format!(
        "SELECT {USER_PROFILE_COLUMNS} FROM user_profiles \
         WHERE project_id = $1 AND (visitor_id = ANY($2::text[]) OR ($3::text IS NOT NULL AND user_id = $3)) \
         ORDER BY last_seen_at DESC \
         LIMIT $4"
    ))
    .bind(project_id)
    .bind(visitor_ids)
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(profiles)
}

async fn graph_aliases(
    db: &PgPool,
    project_id: Uuid,
    visitor_ids: &[String],
    user_id: Option<&str>,
    limit: i64,
) -> AppResult<Vec<UserAlias>> {
    let aliases = sqlx::query_as(
        "SELECT id, project_id, user_id, visitor_id, created_at \
         FROM user_aliases \
         WHERE project_id = $1 AND (visitor_id = ANY($2::text[]) OR ($3::text IS NOT NULL AND user_id = $3)) \
         ORDER BY created_at DESC \
         LIMIT $4",
    )
    .bind(project_id)
    .bind(visitor_ids)
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(aliases)
}

async fn graph_memberships(
    db: &PgPool,
    project_id: Uuid,
    visitor_ids: &[String],
    user_id: Option<&str>,
    account_id: Option<&str>,
    limit: i64,
) -> AppResult<Vec<AccountMembership>> {
    let members = sqlx::query_as(&format!(
        "SELECT {ACCOUNT_MEMBERSHIP_COLUMNS} FROM account_memberships \
         WHERE project_id = $1 AND (visitor_id = ANY($2::text[]) OR ($3::text IS NOT NULL AND user_id = $3) OR ($4::text IS NOT NULL AND account_id = $4)) \
         ORDER BY last_seen_at DESC \
         LIMIT $5"
    ))
    .bind(project_id)
    .bind(visitor_ids)
    .bind(user_id)
    .bind(account_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(members)
}

async fn graph_accounts(
    db: &PgPool,
    project_id: Uuid,
    account_ids: &BTreeSet<String>,
) -> AppResult<Vec<AccountProfile>> {
    if account_ids.is_empty() {
        return Ok(Vec::new());
    }
    let account_ids = account_ids.iter().cloned().collect::<Vec<_>>();
    let accounts = sqlx::query_as(&format!(
        "SELECT {ACCOUNT_PROFILE_COLUMNS} FROM account_profiles \
         WHERE project_id = $1 AND account_id = ANY($2::text[]) \
         ORDER BY last_seen_at DESC"
    ))
    .bind(project_id)
    .bind(&account_ids)
    .fetch_all(db)
    .await?;
    Ok(accounts)
}

async fn upsert_account(
    db: &PgPool,
    project_id: Uuid,
    account_id: &str,
    name: Option<&str>,
    traits: &serde_json::Value,
    seen_at: DateTime<Utc>,
) -> AppResult<AccountProfile> {
    let account = sqlx::query_as(&format!(
        "INSERT INTO account_profiles \
         (project_id, account_id, name, traits, first_seen_at, last_seen_at) \
         VALUES ($1, $2, $3, $4, $5, $5) \
         ON CONFLICT (project_id, account_id) DO UPDATE SET \
             name = COALESCE(EXCLUDED.name, account_profiles.name), \
             traits = account_profiles.traits || EXCLUDED.traits, \
             first_seen_at = LEAST(account_profiles.first_seen_at, EXCLUDED.first_seen_at), \
             last_seen_at = GREATEST(account_profiles.last_seen_at, EXCLUDED.last_seen_at), \
             updated_at = NOW() \
         RETURNING {ACCOUNT_PROFILE_COLUMNS}"
    ))
    .bind(project_id)
    .bind(account_id)
    .bind(name)
    .bind(traits)
    .bind(seen_at)
    .fetch_one(db)
    .await?;

    Ok(account)
}

async fn upsert_account_membership(
    db: &PgPool,
    project_id: Uuid,
    account_id: &str,
    user_id: Option<&str>,
    visitor_id: &str,
    role: Option<&str>,
    seen_at: DateTime<Utc>,
) -> AppResult<AccountMembership> {
    let empty_traits = serde_json::json!({});
    let member = sqlx::query_as(&format!(
        "INSERT INTO account_memberships \
         (project_id, account_id, user_id, visitor_id, role, traits, first_seen_at, last_seen_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7) \
         ON CONFLICT (project_id, account_id, visitor_id) DO UPDATE SET \
             user_id = COALESCE(EXCLUDED.user_id, account_memberships.user_id), \
             role = COALESCE(EXCLUDED.role, account_memberships.role), \
             traits = account_memberships.traits || EXCLUDED.traits, \
             first_seen_at = LEAST(account_memberships.first_seen_at, EXCLUDED.first_seen_at), \
             last_seen_at = GREATEST(account_memberships.last_seen_at, EXCLUDED.last_seen_at), \
             updated_at = NOW() \
         RETURNING {ACCOUNT_MEMBERSHIP_COLUMNS}"
    ))
    .bind(project_id)
    .bind(account_id)
    .bind(user_id)
    .bind(visitor_id)
    .bind(role)
    .bind(&empty_traits)
    .bind(seen_at)
    .fetch_one(db)
    .await?;

    Ok(member)
}

async fn sync_scim_user_to_identity(db: &PgPool, user: &ScimUser) -> AppResult<UserProfile> {
    let visitor_id = format!("scim:{}", user.id);
    let traits = serde_json::json!({
        "source": "scim",
        "scim_user_id": user.id,
        "external_id": user.external_id,
        "active": user.active,
        "display_name": user.display_name,
        "given_name": user.given_name,
        "family_name": user.family_name,
        "emails": user.emails,
        "traits": user.traits,
    });
    identify_user(
        db,
        user.project_id,
        &visitor_id,
        Some(&user.user_name),
        &traits,
        None,
        None,
        None,
        None,
        Utc::now(),
    )
    .await
}

async fn scim_group_members(
    db: &PgPool,
    project_id: Uuid,
    group_id: Uuid,
) -> AppResult<Vec<ScimUser>> {
    let members = sqlx::query_as(&format!(
        "SELECT {SCIM_USER_JOIN_COLUMNS} \
         FROM scim_group_members m \
         JOIN scim_users u ON u.id = m.user_id AND u.project_id = m.project_id \
         WHERE m.project_id = $1 AND m.group_id = $2 \
         ORDER BY u.user_name ASC"
    ))
    .bind(project_id)
    .bind(group_id)
    .fetch_all(db)
    .await?;
    Ok(members)
}

async fn ensure_scim_users_exist(
    db: &PgPool,
    project_id: Uuid,
    member_ids: &[Uuid],
) -> AppResult<()> {
    if member_ids.is_empty() {
        return Ok(());
    }
    let unique_ids = unique_uuids(member_ids);
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM scim_users WHERE project_id = $1 AND id = ANY($2::uuid[])",
    )
    .bind(project_id)
    .bind(&unique_ids)
    .fetch_one(db)
    .await?;
    if count != unique_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "SCIM group members must reference existing SCIM users".to_string(),
        ));
    }
    Ok(())
}

async fn replace_scim_group_members(
    db: &PgPool,
    project_id: Uuid,
    group_id: Uuid,
    member_ids: &[Uuid],
) -> AppResult<()> {
    sqlx::query("DELETE FROM scim_group_members WHERE project_id = $1 AND group_id = $2")
        .bind(project_id)
        .bind(group_id)
        .execute(db)
        .await?;
    for user_id in unique_uuids(member_ids) {
        sqlx::query(
            "INSERT INTO scim_group_members (project_id, group_id, user_id) \
             VALUES ($1, $2, $3) ON CONFLICT (group_id, user_id) DO NOTHING",
        )
        .bind(project_id)
        .bind(group_id)
        .bind(user_id)
        .execute(db)
        .await?;
    }
    Ok(())
}

fn validate_scim_user_input(mut input: ScimUserInput) -> AppResult<ScimUserInput> {
    input.user_name = input.user_name.trim().to_string();
    input.external_id = owned_non_empty(input.external_id);
    input.display_name = owned_non_empty(input.display_name);
    input.given_name = owned_non_empty(input.given_name);
    input.family_name = owned_non_empty(input.family_name);
    input.traits = normalized_json_object(Some(&input.traits), "traits")?;
    input.emails = normalized_json_array(Some(&input.emails), "emails")?;

    if input.user_name.is_empty() {
        return Err(AppError::BadRequest(
            "SCIM user_name is required".to_string(),
        ));
    }
    if input.user_name.len() > 255 {
        return Err(AppError::BadRequest(
            "SCIM user_name must be 255 characters or fewer".to_string(),
        ));
    }
    Ok(input)
}

fn validate_scim_group_input(mut input: ScimGroupInput) -> AppResult<ScimGroupInput> {
    input.display_name = input.display_name.trim().to_string();
    input.external_id = owned_non_empty(input.external_id);
    input.traits = normalized_json_object(Some(&input.traits), "traits")?;
    input.members = unique_uuids(&input.members);

    if input.display_name.is_empty() {
        return Err(AppError::BadRequest(
            "SCIM group display_name is required".to_string(),
        ));
    }
    if input.members.len() > 1000 {
        return Err(AppError::BadRequest(
            "SCIM groups support at most 1000 members".to_string(),
        ));
    }
    Ok(input)
}

fn unique_uuids(values: &[Uuid]) -> Vec<Uuid> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .copied()
        .filter(|value| seen.insert(*value))
        .collect()
}

fn owned_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalized_json_object(
    value: Option<&serde_json::Value>,
    field: &str,
) -> AppResult<serde_json::Value> {
    match value {
        Some(value) if value.is_object() => Ok(value.clone()),
        Some(value) if value.is_null() => Ok(serde_json::json!({})),
        Some(_) => Err(AppError::BadRequest(format!("{field} must be an object"))),
        None => Ok(serde_json::json!({})),
    }
}

fn normalized_json_array(
    value: Option<&serde_json::Value>,
    field: &str,
) -> AppResult<serde_json::Value> {
    match value {
        Some(value) if value.is_array() => Ok(value.clone()),
        Some(value) if value.is_null() => Ok(serde_json::json!([])),
        Some(_) => Err(AppError::BadRequest(format!("{field} must be an array"))),
        None => Ok(serde_json::json!([])),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey {
    source: String,
    target: String,
    edge_type: String,
}

#[derive(Default)]
struct IdentityGraphBuilder {
    nodes: BTreeMap<String, IdentityGraphNode>,
    edge_keys: BTreeSet<EdgeKey>,
    edges: Vec<IdentityGraphEdge>,
}

impl IdentityGraphBuilder {
    fn add_visitor(&mut self, visitor_id: &str, metadata: serde_json::Value) {
        self.add_node("visitor", visitor_id, Some(visitor_id), metadata);
    }

    fn add_user(&mut self, user_id: &str, metadata: serde_json::Value) {
        self.add_node("user", user_id, Some(user_id), metadata);
    }

    fn add_account(&mut self, account_id: &str, name: Option<&str>, metadata: serde_json::Value) {
        self.add_node("account", account_id, name.or(Some(account_id)), metadata);
    }

    fn add_node(
        &mut self,
        node_type: &str,
        key: &str,
        label: Option<&str>,
        metadata: serde_json::Value,
    ) {
        let id = node_id(node_type, key);
        let node = IdentityGraphNode {
            id: id.clone(),
            node_type: node_type.to_string(),
            key: key.to_string(),
            label: label.map(ToOwned::to_owned),
            metadata,
        };

        self.nodes
            .entry(id)
            .and_modify(|existing| {
                if existing.label.is_none() {
                    existing.label = node.label.clone();
                }
                if is_non_empty_object(&node.metadata) {
                    existing.metadata = node.metadata.clone();
                }
            })
            .or_insert(node);
    }

    fn add_edge(
        &mut self,
        source: &str,
        target: &str,
        edge_type: &str,
        metadata: serde_json::Value,
    ) {
        let key = EdgeKey {
            source: source.to_string(),
            target: target.to_string(),
            edge_type: edge_type.to_string(),
        };
        if self.edge_keys.insert(key) {
            self.edges.push(IdentityGraphEdge {
                id: format!("{source}->{edge_type}->{target}"),
                source: source.to_string(),
                target: target.to_string(),
                edge_type: edge_type.to_string(),
                metadata,
            });
        }
    }

    fn finish(self) -> IdentityGraph {
        IdentityGraph {
            nodes: self.nodes.into_values().collect(),
            edges: self.edges,
        }
    }
}

fn node_id(node_type: &str, key: &str) -> String {
    format!("{node_type}:{key}")
}

fn is_non_empty_object(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|object| !object.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        node_id, normalize_optional, normalized_json_array, normalized_json_object,
        validate_scim_group_input, validate_scim_user_input, IdentityGraphBuilder, ScimGroupInput,
        ScimUserInput,
    };
    use uuid::Uuid;

    #[test]
    fn normalizes_optional_identity_fields() {
        assert_eq!(normalize_optional(Some("  acct_123  ")), Some("acct_123"));
        assert_eq!(normalize_optional(Some("   ")), None);
        assert_eq!(normalize_optional(None), None);
    }

    #[test]
    fn account_traits_must_be_objects() {
        assert!(normalized_json_object(
            Some(&serde_json::json!({ "plan": "pro" })),
            "account_traits"
        )
        .is_ok());
        assert!(normalized_json_object(Some(&serde_json::json!([])), "account_traits").is_err());
        assert!(
            normalized_json_array(Some(&serde_json::json!(["a@example.com"])), "emails").is_ok()
        );
        assert!(normalized_json_array(Some(&serde_json::json!({})), "emails").is_err());
    }

    #[test]
    fn validates_scim_user_inputs() {
        let input = validate_scim_user_input(ScimUserInput {
            user_name: " alice@example.com ".to_string(),
            external_id: Some(" ext-1 ".to_string()),
            active: true,
            display_name: Some(" Alice Example ".to_string()),
            given_name: Some(" Alice ".to_string()),
            family_name: Some(" Example ".to_string()),
            emails: serde_json::json!([{ "value": "alice@example.com", "primary": true }]),
            traits: serde_json::json!({ "department": "Engineering" }),
        })
        .expect("valid scim user");

        assert_eq!(input.user_name, "alice@example.com");
        assert_eq!(input.external_id.as_deref(), Some("ext-1"));
        assert_eq!(input.display_name.as_deref(), Some("Alice Example"));
        assert!(validate_scim_user_input(ScimUserInput {
            user_name: " ".to_string(),
            external_id: None,
            active: true,
            display_name: None,
            given_name: None,
            family_name: None,
            emails: serde_json::json!([]),
            traits: serde_json::json!({}),
        })
        .is_err());
    }

    #[test]
    fn validates_scim_group_inputs_and_deduplicates_members() {
        let user_id = Uuid::new_v4();
        let input = validate_scim_group_input(ScimGroupInput {
            display_name: " Engineering ".to_string(),
            external_id: Some(" group-1 ".to_string()),
            traits: serde_json::json!({ "cost_center": "eng" }),
            members: vec![user_id, user_id],
        })
        .expect("valid scim group");

        assert_eq!(input.display_name, "Engineering");
        assert_eq!(input.external_id.as_deref(), Some("group-1"));
        assert_eq!(input.members, vec![user_id]);
    }

    #[test]
    fn identity_graph_builder_deduplicates_nodes_and_edges() {
        let mut builder = IdentityGraphBuilder::default();
        builder.add_visitor("v_123", serde_json::json!({}));
        builder.add_visitor("v_123", serde_json::json!({ "last_seen_at": "now" }));
        builder.add_user("user_123", serde_json::json!({}));
        builder.add_account(
            "acct_123",
            Some("Acme"),
            serde_json::json!({ "plan": "pro" }),
        );
        builder.add_edge(
            &node_id("visitor", "v_123"),
            &node_id("user", "user_123"),
            "alias",
            serde_json::json!({}),
        );
        builder.add_edge(
            &node_id("visitor", "v_123"),
            &node_id("user", "user_123"),
            "alias",
            serde_json::json!({ "duplicate": true }),
        );

        let graph = builder.finish();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.id == "visitor:v_123" && node.metadata["last_seen_at"] == "now"));
    }
}
