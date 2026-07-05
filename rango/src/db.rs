//! Rango Database Module
//!
//! Connection is configured via `RangoConfig.database` — never passed manually.

#[cfg(feature = "db")]
use sqlx::{Pool, any::AnyArguments, query::Query, query::QueryAs};
use std::sync::OnceLock;

#[cfg(feature = "db")]
use crate::state::{DatabaseBackend, DatabaseConfig};

#[cfg(feature = "db")]
static DB_POOL: OnceLock<Pool<sqlx::Any>> = OnceLock::new();

#[cfg(feature = "db")]
static DB_BACKEND: OnceLock<DatabaseBackend> = OnceLock::new();

// ─── Connection ───────────────────────────────────────────────────────────────

/// Initialize the DB pool from a raw URL (low-level, prefer `init_db_with_config`).
#[cfg(feature = "db")]
pub async fn init_db(database_url: &str) -> Result<(), sqlx::Error> {
    sqlx::any::install_default_drivers();
    let pool = sqlx::AnyPool::connect(database_url).await?;
    let _ = DB_POOL.set(pool);
    let _ = DB_BACKEND.set(DatabaseBackend::from_url(database_url));
    tracing::info!("🗄️  Rango DB connected: {}", database_url);
    Ok(())
}

/// Initialize the DB pool from `DatabaseConfig` (used by `RangoBuilder`).
/// Respects `max_connections`, `min_connections`, and `connect_timeout_secs`.
#[cfg(feature = "db")]
pub async fn init_db_with_config(cfg: &DatabaseConfig) -> Result<(), sqlx::Error> {
    use sqlx::any::AnyPoolOptions;
    use std::time::Duration;

    sqlx::any::install_default_drivers();

    let pool = AnyPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.connect_timeout_secs))
        .connect(&cfg.url)
        .await?;

    let _ = DB_POOL.set(pool);
    let _ = DB_BACKEND.set(cfg.backend());
    tracing::info!(
        "🗄️  Rango DB connected [{}]: {}",
        cfg.backend().name(),
        cfg.url
    );
    Ok(())
}

/// Access the global database pool.
/// Returns an error instead of panicking if the DB is not initialized.
#[cfg(feature = "db")]
pub fn db() -> crate::RangoResult<&'static Pool<sqlx::Any>> {
    DB_POOL.get().ok_or_else(|| {
        crate::error::RangoError::DatabaseNotInitialized(
            "set RangoConfig.database before calling rango::start()".to_string(),
        )
    })
}

/// Access the globally configured database backend.
/// Returns an error instead of panicking if the DB is not initialized.
#[cfg(feature = "db")]
pub fn backend() -> crate::RangoResult<&'static DatabaseBackend> {
    DB_BACKEND.get().ok_or_else(|| {
        crate::error::RangoError::DatabaseNotInitialized(
            "DB backend not set — call init_db / init_db_with_config first".to_string(),
        )
    })
}

/// Build the correct positional placeholder for the current backend.
/// - Postgres  → `$1`, `$2`, ...
/// - SQLite / MySQL → `?`
#[cfg(feature = "db")]
pub fn placeholder(backend: &DatabaseBackend, index: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${}", index),
        DatabaseBackend::Sqlite | DatabaseBackend::Mysql | DatabaseBackend::Any => "?".to_string(),
    }
}

// ─── Raw query helpers ────────────────────────────────────────────────────────

#[cfg(feature = "db")]
pub async fn execute(sql: &str) -> crate::RangoResult<u64> {
    let pool = db()?;
    let result = sqlx::query(sql)
        .execute(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(result.rows_affected())
}

#[cfg(feature = "db")]
pub fn query(sql: &str) -> Query<'_, sqlx::Any, AnyArguments<'_>> {
    sqlx::query(sql)
}

#[cfg(feature = "db")]
pub fn query_as<T>(sql: &str) -> QueryAs<'_, sqlx::Any, T, AnyArguments<'_>>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow>,
{
    sqlx::query_as(sql)
}

// ─── Transactions ─────────────────────────────────────────────────────────────

#[cfg(feature = "db")]
pub async fn with_transaction<F, T>(f: F) -> crate::RangoResult<T>
where
    F: for<'c> FnOnce(
            &'c mut sqlx::Transaction<'static, sqlx::Any>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::RangoResult<T>> + Send + 'c>>
        + Send,
    T: Send,
{
    let pool = db()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    match f(&mut tx).await {
        Ok(result) => {
            tx.commit()
                .await
                .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
            Ok(result)
        }
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}

// ─── Aggregations ─────────────────────────────────────────────────────────────

#[cfg(feature = "db")]
pub async fn aggregate(sql: &str) -> crate::RangoResult<Option<i64>> {
    let pool = db()?;
    let row: Option<(Option<i64>,)> = sqlx::query_as(sql)
        .fetch_optional(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(row.and_then(|(v,)| v))
}

#[cfg(feature = "db")]
pub async fn aggregate_float(sql: &str) -> crate::RangoResult<Option<f64>> {
    let pool = db()?;
    let row: Option<(Option<f64>,)> = sqlx::query_as(sql)
        .fetch_optional(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(row.and_then(|(v,)| v))
}

// ─── QuerySet ─────────────────────────────────────────────────────────────────

#[cfg(feature = "db")]
pub struct QuerySet<T> {
    table: String,
    conditions: Vec<String>,
    order_by: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
    select_fields: Option<String>,
    _phantom: std::marker::PhantomData<T>,
}

#[cfg(feature = "db")]
impl<T> QuerySet<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Send + Unpin,
{
    pub fn new(table: &str) -> Self {
        QuerySet {
            table: table.to_string(),
            conditions: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
            select_fields: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn filter(mut self, condition: &str) -> Self {
        self.conditions.push(condition.to_string());
        self
    }

    pub fn exclude(mut self, condition: &str) -> Self {
        self.conditions.push(format!("NOT ({})", condition));
        self
    }

    pub fn order_by(mut self, order: &str) -> Self {
        self.order_by = Some(order.to_string());
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn only(mut self, fields: &str) -> Self {
        self.select_fields = Some(fields.to_string());
        self
    }

    fn build_query(&self) -> String {
        let select = self.select_fields.clone().unwrap_or_else(|| "*".to_string());
        let mut q = format!("SELECT {} FROM {}", select, self.table);
        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        if let Some(ref order) = self.order_by {
            q.push_str(&format!(" ORDER BY {}", order));
        }
        if let Some(lim) = self.limit {
            q.push_str(&format!(" LIMIT {}", lim));
        }
        if let Some(off) = self.offset {
            q.push_str(&format!(" OFFSET {}", off));
        }
        q
    }

    pub async fn all(self) -> crate::RangoResult<Vec<T>> {
        let q = self.build_query();
        tracing::debug!("QuerySet::all — {}", q);
        let pool = db()?;
        sqlx::query_as::<_, T>(&q)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))
    }

    pub async fn first(self) -> crate::RangoResult<Option<T>> {
        let qs = self.limit(1);
        let q = qs.build_query();
        tracing::debug!("QuerySet::first — {}", q);
        let pool = db()?;
        sqlx::query_as::<_, T>(&q)
            .fetch_optional(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))
    }

    pub async fn count(self) -> crate::RangoResult<i64> {
        let mut q = format!("SELECT COUNT(*) FROM {}", self.table);
        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        tracing::debug!("QuerySet::count — {}", q);
        let pool = db()?;
        let row: (i64,) = sqlx::query_as(&q)
            .fetch_one(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        Ok(row.0)
    }

    pub async fn exists(self) -> crate::RangoResult<bool> {
        Ok(self.count().await? > 0)
    }

    pub async fn delete(self) -> crate::RangoResult<u64> {
        let mut q = format!("DELETE FROM {}", self.table);
        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        tracing::debug!("QuerySet::delete — {}", q);
        let pool = db()?;
        let result = sqlx::query(&q)
            .execute(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected())
    }

    pub async fn paginate(self, page: u64, per_page: u64) -> crate::RangoResult<(Vec<T>, bool)> {
        let table = self.table.clone();
        let conditions = self.conditions.clone();
        let offset = (page - 1) * per_page;

        let mut count_qs = QuerySet::<T>::new(&table);
        count_qs.conditions = conditions;
        let total = count_qs.count().await?;

        let items = self.limit(per_page).offset(offset).all().await?;
        let has_next = (offset + items.len() as u64) < total as u64;
        Ok((items, has_next))
    }
}

// ─── RangoModel trait ─────────────────────────────────────────────────────────

#[cfg(feature = "db")]
#[::axum::async_trait]
pub trait RangoModel:
    Sized + Send + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Clone
{
    fn table_name() -> &'static str;
    async fn save(&mut self) -> crate::RangoResult<()>;
    async fn delete(&self) -> crate::RangoResult<u64>;

    fn objects() -> QuerySet<Self> {
        QuerySet::new(Self::table_name())
    }

    async fn all() -> crate::RangoResult<Vec<Self>> {
        Self::objects().all().await
    }

    async fn get_by_id(id: i64) -> crate::RangoResult<Option<Self>> {
        let b = backend()?;
        let ph = placeholder(b, 1);
        let q = format!("SELECT * FROM {} WHERE id = {}", Self::table_name(), ph);
        let pool = db()?;
        sqlx::query_as::<_, Self>(&q)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))
    }

    async fn get_or_404(id: i64) -> crate::RangoResult<Self> {
        Self::get_by_id(id).await?.ok_or_else(|| {
            crate::error::RangoError::NotFound(format!("{} not found", Self::table_name()))
        })
    }

    async fn get(condition: &str) -> crate::RangoResult<Option<Self>> {
        Self::objects().filter(condition).first().await
    }

    async fn filter(condition: &str) -> crate::RangoResult<Vec<Self>> {
        Self::objects().filter(condition).all().await
    }

    async fn count() -> crate::RangoResult<i64> {
        Self::objects().count().await
    }

    async fn delete_by_id(id: i64) -> crate::RangoResult<u64> {
        let b = backend()?;
        let ph = placeholder(b, 1);
        let q = format!("DELETE FROM {} WHERE id = {}", Self::table_name(), ph);
        let pool = db()?;
        let result = sqlx::query(&q)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn get_or_create(
        condition: &str,
        mut default_instance: Self,
    ) -> crate::RangoResult<(Self, bool)> {
        if let Some(existing) = Self::get(condition).await? {
            return Ok((existing, false));
        }
        default_instance.save().await?;
        Ok((default_instance, true))
    }

    fn create_table_sql() -> crate::RangoResult<String> {
        let pk = match backend()? {
            DatabaseBackend::Postgres => "id SERIAL PRIMARY KEY",
            DatabaseBackend::Mysql => "id INT AUTO_INCREMENT PRIMARY KEY",
            DatabaseBackend::Sqlite | DatabaseBackend::Any => {
                "id INTEGER PRIMARY KEY AUTOINCREMENT"
            }
        };
        Ok(format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            Self::table_name(),
            pk
        ))
    }

    async fn create_table() -> crate::RangoResult<()> {
        let sql = Self::create_table_sql()?;
        let pool = db()?;
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        tracing::info!("✅ Table '{}' ready.", Self::table_name());
        Ok(())
    }
}

// ─── Migrations ───────────────────────────────────────────────────────────────

#[cfg(feature = "db")]
pub async fn run_migrations(migrations_path: &str) -> Result<(), sqlx::Error> {
    use sqlx::migrate::Migrator;
    use std::path::Path;
    let pool = db().map_err(|_| sqlx::Error::PoolClosed)?;
    let migrator = Migrator::new(Path::new(migrations_path)).await?;
    migrator.run(pool).await?;
    tracing::info!("✅ Migrations applied from {}", migrations_path);
    Ok(())
}

// ─── Schema ───────────────────────────────────────────────────────────────────

#[cfg(feature = "db")]
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub unique: bool,
}

#[cfg(feature = "db")]
impl ColumnDef {
    pub fn to_sql(&self) -> String {
        let mut s = format!("{} {}", self.name, self.sql_type);
        if !self.nullable {
            s.push_str(" NOT NULL");
        }
        if self.unique {
            s.push_str(" UNIQUE");
        }
        if let Some(ref d) = self.default {
            s.push_str(&format!(" DEFAULT {}", d));
        }
        s
    }
}

#[cfg(feature = "db")]
pub trait RangoSchema {
    fn columns() -> Vec<ColumnDef>;
    fn generate_migration_sql() -> String;
}

// ─── Admin Panel Metadata and Operations ──────────────────────────────────────

#[cfg(feature = "db")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AdminField {
    pub name: String,
    pub field_type: String,
    pub editable: bool,
}

#[cfg(feature = "db")]
pub trait RangoAdminMetadata {
    fn model_name() -> &'static str;
    fn fields() -> Vec<AdminField>;
    fn to_json_value(&self) -> serde_json::Value;
    fn from_form(
        form_data: &std::collections::HashMap<String, String>,
    ) -> Result<Self, String>
    where
        Self: Sized;
    fn update_from_form(
        &mut self,
        form_data: &std::collections::HashMap<String, String>,
    ) -> Result<(), String>;
}

#[cfg(feature = "db")]
#[::axum::async_trait]
pub trait RangoAdminOps: Send + Sync {
    fn model_name(&self) -> &'static str;
    fn fields(&self) -> Vec<AdminField>;
    async fn list(&self) -> crate::RangoResult<Vec<serde_json::Value>>;
    async fn get(&self, id: i64) -> crate::RangoResult<Option<serde_json::Value>>;
    async fn save(
        &self,
        id: Option<i64>,
        form_data: &std::collections::HashMap<String, String>,
    ) -> crate::RangoResult<()>;
    async fn delete(&self, id: i64) -> crate::RangoResult<()>;
}

#[cfg(feature = "db")]
pub struct ModelAdmin<T> {
    _marker: std::marker::PhantomData<T>,
}

#[cfg(feature = "db")]
impl<T> ModelAdmin<T> {
    pub fn new() -> Self {
        ModelAdmin {
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "db")]
impl<T> Default for ModelAdmin<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "db")]
#[::axum::async_trait]
impl<T> RangoAdminOps for ModelAdmin<T>
where
    T: RangoModel + RangoAdminMetadata + Send + Sync + 'static,
{
    fn model_name(&self) -> &'static str {
        T::model_name()
    }

    fn fields(&self) -> Vec<AdminField> {
        T::fields()
    }

    async fn list(&self) -> crate::RangoResult<Vec<serde_json::Value>> {
        let items = T::all().await?;
        Ok(items.iter().map(|item| item.to_json_value()).collect())
    }

    async fn get(&self, id: i64) -> crate::RangoResult<Option<serde_json::Value>> {
        let item = T::get_by_id(id).await?;
        Ok(item.map(|i| i.to_json_value()))
    }

    async fn save(
        &self,
        id: Option<i64>,
        form_data: &std::collections::HashMap<String, String>,
    ) -> crate::RangoResult<()> {
        if let Some(id_val) = id {
            let mut item = T::get_by_id(id_val)
                .await?
                .ok_or_else(|| {
                    crate::error::RangoError::NotFound(format!(
                        "Record not found with ID {}",
                        id_val
                    ))
                })?;
            item.update_from_form(form_data)
                .map_err(|e| crate::error::RangoError::Internal(e))?;
            item.save().await?;
        } else {
            let mut item = T::from_form(form_data)
                .map_err(|e| crate::error::RangoError::Internal(e))?;
            item.save().await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> crate::RangoResult<()> {
        T::delete_by_id(id).await?;
        Ok(())
    }
}
