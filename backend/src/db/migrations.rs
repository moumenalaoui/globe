//! Additive schema migrations for databases that already exist.
//!
//! `schema::create_tables` is `CREATE TABLE IF NOT EXISTS` throughout, which
//! means it only ever describes a *fresh* database. Once a `mena_ai.db` exists,
//! editing a `CREATE TABLE` there has no effect — the table is already present,
//! so the new column is silently never added and every read of it fails at
//! runtime. That is fine while the only database is a local one you can delete
//! and re-seed, and stops being fine the moment there is deployed state.
//!
//! This module closes that gap for the common case: adding a nullable column.
//! `ADD COLUMN` is the one `ALTER TABLE` SQLite does cheaply — it rewrites no
//! rows, so running it against a large table is effectively free. Anything
//! structural (dropping a column, changing a type, adding a constraint) still
//! needs the 12-step table-rebuild dance and is deliberately out of scope here.
//!
//! Adding a column is a two-line change: add it to the `CREATE TABLE` in
//! `schema.rs` so fresh databases get it, and add it to `COLUMNS` below so
//! existing ones do too. Both are required — they serve different databases.

use anyhow::{Context, Result};
use rusqlite::Connection;

/// (table, column, type + any default) — applied in order, each skipped if the
/// column is already present. Entries stay here permanently; they are the
/// record of what existing databases need, not a changelog to prune.
const COLUMNS: &[(&str, &str, &str)] = &[
    // Per-transport bridge estimates from userstats-bridge-combined.csv.
    // Tor publishes these as a low/high interval rather than a point estimate
    // (they are modelled from directory requests, not counts), so both bounds
    // are stored and the midpoint is derived at display time. Storing only the
    // midpoint would discard the published uncertainty.
    ("tor_metrics", "obfs4_low", "INTEGER"),
    ("tor_metrics", "obfs4_high", "INTEGER"),
    ("tor_metrics", "snowflake_low", "INTEGER"),
    ("tor_metrics", "snowflake_high", "INTEGER"),
    ("tor_metrics", "webtunnel_low", "INTEGER"),
    ("tor_metrics", "webtunnel_high", "INTEGER"),
];

/// Brings an existing database up to the current schema. Idempotent: safe to
/// run on every boot, and a no-op on a database `create_tables` just built.
pub fn run(conn: &Connection) -> Result<()> {
    let mut added = 0;
    for (table, column, decl) in COLUMNS {
        if add_column_if_missing(conn, table, column, decl)? {
            added += 1;
        }
    }
    if added > 0 {
        println!("migrations: added {added} column(s) to existing tables.");
    }
    Ok(())
}

/// Returns true if the column was added, false if it was already there.
///
/// The existence check is `PRAGMA table_info` rather than catching the error
/// from a blind `ADD COLUMN`: SQLite reports a duplicate column as a generic
/// `SqliteFailure`, which is indistinguishable from a real failure (bad type,
/// missing table) without string-matching the message.
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<bool> {
    if !table_exists(conn, table)? {
        // create_tables runs first, so a missing table means the entry names a
        // table that no longer exists — a stale migration, worth failing on
        // rather than skipping silently.
        anyhow::bail!("migration targets unknown table `{table}`");
    }
    if column_exists(conn, table, column)? {
        return Ok(false);
    }
    // Identifiers can't be bound as parameters, and these are compile-time
    // constants from COLUMNS above, never user input.
    conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))
        .with_context(|| format!("failed to add column `{column}` to `{table}`"))?;
    Ok(true)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        rusqlite::params![table],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        // table_info columns: cid, name, type, notnull, dflt_value, pk
        let name: String = row.get(1)?;
        if name.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}
