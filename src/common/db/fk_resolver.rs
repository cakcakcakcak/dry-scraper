use std::collections::HashSet;

use crate::common::{
    app_context::AppContext,
    db::{DbContext, DbEntity, DbEntityVecExt, PrimaryKey},
    errors::DSError,
};

/// Resolves missing foreign keys for a batch of entities.
///
/// This helper is called from the orchestrator layer to ensure all referenced
/// foreign keys exist in the database before upserting a batch of entities.
///
/// # Behavior
///
/// 1. Extracts all unique FK references from the entities via `foreign_keys()`
/// 2. Checks which ones are missing from the key cache
/// 3. If any are missing, calls the provided closure to fetch them
/// 4. Upserts the fetched FK entities (ensuring they exist in the DB)
/// 5. Returns Ok if all FKs are now satisfied, or if there were no FKs to resolve
///
/// # Usage
///
/// ```rust,ignore
/// // After fetching games, before upserting them
/// let games = nhl_api.fetch_and_parse_games(&endpoint, db_context).await?;
///
/// // Ensure all referenced teams exist
/// resolve_foreign_keys(
///     &games,
///     app_context,
///     db_context,
///     || async {
///         // Fetch teams by the specific team IDs referenced in games
///         let team_ids: HashSet<i32> = games
///             .iter()
///             .flat_map(|g| vec![g.home_team_id, g.away_team_id])
///             .collect();
///         nhl_api.fetch_teams_by_ids(db_context, team_ids).await
///     },
/// ).await?;
///
/// // Now safe to upsert games
/// games.upsert_all(app_context, db_context).await?;
/// ```
///
/// # Type Parameters
///
/// - `T`: The entity type whose FKs we're resolving
/// - `F`: A closure that returns a future
/// - `Fut`: The future returned by the closure, yielding a Vec of FK entities
///
pub async fn resolve_foreign_keys<T, F, Fut>(
    entities: &[T],
    app_context: &AppContext,
    db_context: &DbContext,
    fetch_missing_fks: F,
) -> Result<(), DSError>
where
    T: DbEntity,
    F: FnOnce(Vec<T::Pk>) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<T>, DSError>>,
{
    let type_name = T::type_name();

    // 1. Collect all unique FK references from entities
    let mut all_fk_keys: HashSet<T::Pk> = HashSet::new();
    for entity in entities {
        for fk in entity.foreign_keys() {
            all_fk_keys.insert(fk);
        }
    }

    if all_fk_keys.is_empty() {
        tracing::debug!("`{type_name}` entities have no foreign keys to resolve");
        return Ok(());
    }

    // 2. Filter to find missing ones (not in key cache)
    let missing: Vec<T::Pk> = all_fk_keys
        .into_iter()
        .filter(|fk| !db_context.key_cache.contains(&fk.cache_key()))
        .collect();

    if missing.is_empty() {
        tracing::debug!("All foreign keys for `{type_name}` already in key cache");
        return Ok(());
    }

    tracing::info!(
        "Found {} missing foreign keys for `{type_name}`, fetching...",
        missing.len()
    );

    // 3. Fetch the missing FK entities
    let fetched_entities = fetch_missing_fks(missing).await?;

    if fetched_entities.is_empty() {
        tracing::warn!("FK fetch returned no entities for `{type_name}`");
        return Ok(());
    }

    // 4. Upsert them to ensure they exist in the DB
    let upserted = fetched_entities.upsert_all(app_context, db_context).await;

    let success_count = upserted.iter().filter(|r| r.is_some()).count();
    tracing::info!(
        "Successfully resolved {}/{} missing foreign keys for `{type_name}`",
        success_count,
        upserted.len()
    );

    Ok(())
}
