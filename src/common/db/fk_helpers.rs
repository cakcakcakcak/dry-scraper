use std::collections::{HashMap, HashSet};

use crate::common::db::{CacheKey, DbContext, DbEntity};

/// Extract all foreign keys from entities and return those missing from the cache.
///
/// This helper collects FK references from a batch of entities, checks which ones
/// are not present in the key cache, and returns them grouped for fetching.
///
/// # Example
///
/// ```rust,ignore
/// let games = nhl_api.fetch_games(...).await?;
/// let missing = find_missing_foreign_keys(&games, db_context);
/// // missing contains CacheKeys for teams, seasons, etc. that need to be fetched
/// ```
pub fn find_missing_foreign_keys<T>(entities: &[T], db_context: &DbContext) -> Vec<CacheKey>
where
    T: DbEntity,
{
    let mut all_fks: HashSet<CacheKey> = HashSet::new();

    // Collect all unique FK references from entities
    for entity in entities {
        all_fks.extend(entity.foreign_keys());
    }

    // Filter to only those not in cache
    all_fks
        .into_iter()
        .filter(|ck| !db_context.key_cache.contains(ck))
        .collect()
}

/// Group cache keys by their table name for batch fetching.
///
/// Returns a HashMap where keys are table names and values are vectors of IDs
/// that need to be fetched for that table.
///
/// # Example
///
/// ```rust,ignore
/// let missing = find_missing_foreign_keys(&games, db_context);
/// let grouped = group_cache_keys_by_table(&missing);
/// // grouped might be: {"team" => ["1", "5"], "season" => ["20232024"]}
///
/// // Then fetch each group:
/// if let Some(team_ids) = grouped.get("team") {
///     let teams = fetch_teams_by_ids(team_ids).await?;
///     teams.upsert_all(app_context, db_context).await;
/// }
/// ```
pub fn group_cache_keys_by_table(keys: &[CacheKey]) -> HashMap<String, Vec<String>> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();

    for key in keys {
        grouped
            .entry(key.table.to_string())
            .or_default()
            .push(key.id.clone());
    }

    grouped
}

/// Check if all foreign keys for a batch of entities are present in the cache.
///
/// Returns true if all FKs are cached (safe to upsert), false if any are missing.
///
/// # Example
///
/// ```rust,ignore
/// if !all_foreign_keys_cached(&games, db_context) {
///     // Need to fetch missing FKs first
///     let missing = find_missing_foreign_keys(&games, db_context);
///     // ... fetch and upsert missing entities
/// }
/// games.upsert_all(app_context, db_context).await;
/// ```
pub fn all_foreign_keys_cached<T>(entities: &[T], db_context: &DbContext) -> bool
where
    T: DbEntity,
{
    for entity in entities {
        for fk in entity.foreign_keys() {
            if !db_context.key_cache.contains(&fk) {
                return false;
            }
        }
    }
    true
}
