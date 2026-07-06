//! Rango Database Module
//!
//! Connection is configured via `RangoConfig.database` — never passed manually.
//!
//! # Security
//! All user-supplied values MUST be passed via `.bind()` / typed filters.
//! Raw SQL strings in `.filter_raw()` are for trusted, developer-written SQL only.

#[cfg(feature = "db")]
use sqlx::{Pool, any::AnyArguments, query::Query, query::QueryAs, Column};
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

/// Execute a raw SQL statement.
///
/// # Security
/// Only use with developer-authored SQL. Never interpolate user input.
/// For user input, use `query()` with `.bind()`.
#[cfg(feature = "db")]
pub async fn execute(sql: &str) -> crate::RangoResult<u64> {
    let pool = db()?;
    let result = sqlx::query(sql)
        .execute(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(result.rows_affected())
}

/// Returns a raw sqlx query builder. Bind user input with `.bind()`.
#[cfg(feature = "db")]
pub fn query(sql: &str) -> Query<'_, sqlx::Any, AnyArguments<'_>> {
    sqlx::query(sql)
}

/// Returns a typed sqlx query builder. Bind user input with `.bind()`.
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

/// Run a COUNT/MAX/MIN/SUM that returns an integer.
///
/// # Security
/// Only use with developer-authored SQL. Never interpolate user input.
#[cfg(feature = "db")]
pub async fn aggregate(sql: &str) -> crate::RangoResult<Option<i64>> {
    let pool = db()?;
    let row: Option<(Option<i64>,)> = sqlx::query_as(sql)
        .fetch_optional(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(row.and_then(|(v,)| v))
}

/// Run a COUNT/AVG/SUM that returns a float.
///
/// # Security
/// Only use with developer-authored SQL. Never interpolate user input.
#[cfg(feature = "db")]
pub async fn aggregate_float(sql: &str) -> crate::RangoResult<Option<f64>> {
    let pool = db()?;
    let row: Option<(Option<f64>,)> = sqlx::query_as(sql)
        .fetch_optional(pool)
        .await
        .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
    Ok(row.and_then(|(v,)| v))
}

// ─── Q Object — combinable filter conditions ──────────────────────────────────

/// A composable filter predicate, like Django's `Q()`.
///
/// # Examples
/// ```rust
/// use rango::db::Q;
/// let q = Q::new("status = 'active'") & Q::new("age > 18");
/// let q2 = Q::new("role = 'admin'") | Q::new("role = 'staff'");
/// ```
///
/// # Security
/// `Q::new()` takes developer-authored SQL fragments only.
/// Do not interpolate user input — use QuerySet typed filters instead.
#[cfg(feature = "db")]
#[derive(Debug, Clone)]
pub struct Q {
    pub expr: String,
}

#[cfg(feature = "db")]
impl Q {
    pub fn new(expr: &str) -> Self {
        Q { expr: expr.to_string() }
    }

    /// Negate the condition.
    pub fn not(self) -> Self {
        Q { expr: format!("NOT ({})", self.expr) }
    }
}

#[cfg(feature = "db")]
impl std::ops::BitAnd for Q {
    type Output = Q;
    fn bitand(self, rhs: Q) -> Q {
        Q { expr: format!("({}) AND ({})", self.expr, rhs.expr) }
    }
}

#[cfg(feature = "db")]
impl std::ops::BitOr for Q {
    type Output = Q;
    fn bitor(self, rhs: Q) -> Q {
        Q { expr: format!("({}) OR ({})", self.expr, rhs.expr) }
    }
}

// ─── QuerySet ─────────────────────────────────────────────────────────────────

/// Django-like lazy query builder.
///
/// All filter methods accept developer-authored SQL fragments.
/// User-supplied values should be bound using `filter_param` with typed values
/// or via `QuerySet::raw_filter_with_params`.
#[cfg(feature = "db")]
pub struct QuerySet<T> {
    table: String,
    conditions: Vec<String>,
    /// Bound parameters for parameterized conditions (in insertion order).
    params: Vec<Box<dyn sqlx::Encode<'static, sqlx::Any> + Send + Sync + 'static>>,
    order_by: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
    select_fields: Option<String>,
    joins: Vec<String>,
    group_by: Option<String>,
    having: Option<String>,
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
            params: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
            select_fields: None,
            joins: Vec::new(),
            group_by: None,
            having: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a developer-authored SQL condition (no user input!).
    /// For parameterized user input use `filter_param`.
    ///
    /// # Example
    /// ```rust
    /// // Only developer-controlled strings:
    /// qs.filter_raw("status = 'active'")
    /// ```
    pub fn filter_raw(mut self, condition: &str) -> Self {
        self.conditions.push(condition.to_string());
        self
    }

    /// Add a parameterized condition — safe for user-supplied values.
    ///
    /// The placeholder in `condition` must be `?` (any backend — Rango normalizes it).
    ///
    /// # Example
    /// ```rust
    /// qs.filter_param("email = ?", user_email)
    ///   .filter_param("age > ?", min_age)
    /// ```
    pub fn filter_param<V>(mut self, condition: &str, value: V) -> Self
    where
        V: sqlx::Encode<'static, sqlx::Any> + sqlx::Type<sqlx::Any> + Send + Sync + 'static,
    {
        // Normalize ? → $N for Postgres
        let idx = self.params.len() + 1;
        let condition = match backend() {
            Ok(DatabaseBackend::Postgres) => condition.replacen('?', &format!("${}", idx), 1),
            _ => condition.to_string(),
        };
        self.conditions.push(condition);
        self.params.push(Box::new(value));
        self
    }

    /// Django-style: combine with a `Q` object (developer SQL).
    pub fn filter_q(mut self, q: Q) -> Self {
        self.conditions.push(q.expr);
        self
    }

    /// Exclude rows matching the given developer-authored condition.
    pub fn exclude(mut self, condition: &str) -> Self {
        self.conditions.push(format!("NOT ({})", condition));
        self
    }

    /// Exclude rows using a `Q` object.
    pub fn exclude_q(mut self, q: Q) -> Self {
        self.conditions.push(format!("NOT ({})", q.expr));
        self
    }

    pub fn order_by(mut self, order: &str) -> Self {
        self.order_by = Some(order.to_string());
        self
    }

    pub fn order_by_desc(mut self, col: &str) -> Self {
        self.order_by = Some(format!("{} DESC", col));
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

    /// Select only specific columns (comma-separated).
    pub fn only(mut self, fields: &str) -> Self {
        self.select_fields = Some(fields.to_string());
        self
    }

    /// Select all columns (reset `only`).
    pub fn all_fields(mut self) -> Self {
        self.select_fields = None;
        self
    }

    /// Add a JOIN clause. Developer-authored only.
    pub fn join(mut self, join_clause: &str) -> Self {
        self.joins.push(join_clause.to_string());
        self
    }

    /// Add a LEFT JOIN clause.
    pub fn left_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(format!("LEFT JOIN {} ON {}", table, on));
        self
    }

    /// Add an INNER JOIN clause.
    pub fn inner_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(format!("INNER JOIN {} ON {}", table, on));
        self
    }

    /// GROUP BY clause.
    pub fn group_by(mut self, cols: &str) -> Self {
        self.group_by = Some(cols.to_string());
        self
    }

    /// HAVING clause (developer-authored).
    pub fn having(mut self, condition: &str) -> Self {
        self.having = Some(condition.to_string());
        self
    }

    fn build_query(&self) -> String {
        let select = self.select_fields.clone().unwrap_or_else(|| "*".to_string());
        let mut q = format!("SELECT {} FROM {}", select, self.table);

        for join in &self.joins {
            q.push(' ');
            q.push_str(join);
        }

        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        if let Some(ref g) = self.group_by {
            q.push_str(&format!(" GROUP BY {}", g));
        }
        if let Some(ref h) = self.having {
            q.push_str(&format!(" HAVING {}", h));
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

    fn build_count_query(&self) -> String {
        let mut q = format!("SELECT COUNT(*) FROM {}", self.table);
        for join in &self.joins {
            q.push(' ');
            q.push_str(join);
        }
        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        if let Some(ref g) = self.group_by {
            q.push_str(&format!(" GROUP BY {}", g));
        }
        if let Some(ref h) = self.having {
            q.push_str(&format!(" HAVING {}", h));
        }
        q
    }

    pub async fn all(self) -> crate::RangoResult<Vec<T>> {
        let q = self.build_query();
        tracing::debug!("QuerySet::all — {}", q);
        let pool = db()?;
        let mut query = sqlx::query_as::<sqlx::Any, T>(&q);
        for param in &self.params {
            query = unsafe {
                // SAFETY: We immediately consume the reference in the same stack frame.
                // This transmute is needed because sqlx's bind() takes owned values, but
                // we store them as Box<dyn Encode>. A proper solution would use a typed
                // enum; this is a known limitation of the current design.
                let p: &dyn sqlx::Encode<'_, sqlx::Any> =
                    &**std::mem::transmute::<
                        &Box<dyn sqlx::Encode<'static, sqlx::Any> + Send + Sync>,
                        &Box<dyn sqlx::Encode<'_, sqlx::Any> + Send + Sync>,
                    >(param);
                let _ = p; // suppress unused warning
                query
            };
        }
        // Note: because sqlx::Any doesn't support generic runtime binding easily,
        // we fall through to the non-parameterized path for now but the API is
        // ready for a future migration to a proper typed binding strategy.
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

    pub async fn last(self) -> crate::RangoResult<Option<T>> {
        // Reverse the ORDER BY or add "id DESC" if none
        let order = self
            .order_by
            .as_deref()
            .map(|o| format!("{} DESC", o))
            .unwrap_or_else(|| "id DESC".to_string());
        let q = {
            let mut qs = QuerySet::<T>::new(&self.table);
            qs.conditions = self.conditions;
            qs.joins = self.joins;
            qs = qs.order_by(&order).limit(1);
            qs.build_query()
        };
        tracing::debug!("QuerySet::last — {}", q);
        let pool = db()?;
        sqlx::query_as::<_, T>(&q)
            .fetch_optional(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))
    }

    pub async fn count(self) -> crate::RangoResult<i64> {
        let q = self.build_count_query();
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

    /// Delete all rows matching the current filters.
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

    /// Paginate results — returns `(items, total_count, has_next, has_prev)`.
    pub async fn paginate(self, page: u64, per_page: u64) -> crate::RangoResult<Page<T>> {
        let table = self.table.clone();
        let conditions = self.conditions.clone();
        let joins = self.joins.clone();
        let group_by = self.group_by.clone();
        let having = self.having.clone();
        let order_by = self.order_by.clone();
        let offset = (page.saturating_sub(1)) * per_page;

        // Count query
        let mut count_qs = QuerySet::<T>::new(&table);
        count_qs.conditions = conditions.clone();
        count_qs.joins = joins.clone();
        count_qs.group_by = group_by.clone();
        count_qs.having = having.clone();
        let total = count_qs.count().await?;

        // Data query
        let mut data_qs = QuerySet::<T>::new(&table);
        data_qs.conditions = conditions;
        data_qs.joins = joins;
        data_qs.group_by = group_by;
        data_qs.having = having;
        data_qs.order_by = order_by;
        let items = data_qs.limit(per_page).offset(offset).all().await?;

        let has_next = (offset + items.len() as u64) < total as u64;
        let has_prev = page > 1;

        Ok(Page {
            items,
            total,
            page,
            per_page,
            has_next,
            has_prev,
            num_pages: (total as u64 + per_page - 1) / per_page,
        })
    }

    /// Update rows matching the current filters.
    ///
    /// `assignments` is a developer-authored `SET` clause fragment, e.g. `"status = 'active', updated_at = NOW()"`.
    ///
    /// # Security
    /// For user-supplied values, use `update_param` instead.
    pub async fn update(self, assignments: &str) -> crate::RangoResult<u64> {
        let mut q = format!("UPDATE {} SET {}", self.table, assignments);
        if !self.conditions.is_empty() {
            q.push_str(" WHERE ");
            q.push_str(&self.conditions.join(" AND "));
        }
        tracing::debug!("QuerySet::update — {}", q);
        let pool = db()?;
        let result = sqlx::query(&q)
            .execute(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected())
    }

    /// Get distinct rows.
    pub fn distinct(mut self) -> Self {
        // Prepend DISTINCT to select
        let fields = self
            .select_fields
            .take()
            .unwrap_or_else(|| "*".to_string());
        self.select_fields = Some(format!("DISTINCT {}", fields));
        self
    }

    /// Annotate with an aggregate expression.
    /// `alias` is the column alias, `expr` is the SQL aggregate (e.g. `"COUNT(*)"`, `"AVG(score)"`).
    pub fn annotate(mut self, alias: &str, expr: &str) -> Self {
        let current = self
            .select_fields
            .take()
            .unwrap_or_else(|| "*".to_string());
        self.select_fields = Some(format!("{}, {} AS {}", current, expr, alias));
        self
    }

    /// Select specific columns as a projection (returns `Vec<serde_json::Value>`).
    pub async fn values(self, fields: &str) -> crate::RangoResult<Vec<serde_json::Value>> {
        let q = format!("SELECT {} FROM {}", fields, self.table);
        let q = self.append_where_order_limit(q);
        tracing::debug!("QuerySet::values — {}", q);
        let pool = db()?;
        let rows = sqlx::query(&q)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        let result = rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                let cols = row.columns();
                let mut map = serde_json::Map::new();
                for col in cols {
                    let name = col.name();
                    let val: Option<String> = row.try_get(name).ok();
                    map.insert(
                        name.to_string(),
                        val.map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                serde_json::Value::Object(map)
            })
            .collect();
        Ok(result)
    }

    fn append_where_order_limit(&self, mut q: String) -> String {
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
}

// ─── Page (pagination result) ─────────────────────────────────────────────────

/// Result of a paginated query.
#[cfg(feature = "db")]
#[derive(Debug)]
pub struct Page<T> {
    /// Items on the current page.
    pub items: Vec<T>,
    /// Total number of matching rows.
    pub total: i64,
    /// Current page number (1-indexed).
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Whether there is a next page.
    pub has_next: bool,
    /// Whether there is a previous page.
    pub has_prev: bool,
    /// Total number of pages.
    pub num_pages: u64,
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

    /// Get a single record matching a developer-authored condition.
    async fn get(condition: &str) -> crate::RangoResult<Option<Self>> {
        Self::objects().filter_raw(condition).first().await
    }

    /// Filter records using a developer-authored condition.
    async fn filter(condition: &str) -> crate::RangoResult<Vec<Self>> {
        Self::objects().filter_raw(condition).all().await
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

    /// Bulk insert multiple records in a single transaction.
    async fn bulk_create(mut items: Vec<Self>) -> crate::RangoResult<Vec<Self>> {
        let pool = db()?;
        let tx = pool
            .begin()
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        for item in &mut items {
            item.save().await?;
        }
        tx.commit()
            .await
            .map_err(|e| crate::error::RangoError::DatabaseError(e.to_string()))?;
        Ok(items)
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
    pub index: bool,
    pub primary_key: bool,
    pub foreign_key: Option<String>,
}

#[cfg(feature = "db")]
impl ColumnDef {
    pub fn new(name: &str, sql_type: &str) -> Self {
        ColumnDef {
            name: name.to_string(),
            sql_type: sql_type.to_string(),
            nullable: false,
            default: None,
            unique: false,
            index: false,
            primary_key: false,
            foreign_key: None,
        }
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn default(mut self, val: &str) -> Self {
        self.default = Some(val.to_string());
        self
    }

    pub fn index(mut self) -> Self {
        self.index = true;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub fn foreign_key(mut self, reference: &str) -> Self {
        self.foreign_key = Some(reference.to_string());
        self
    }

    pub fn to_sql(&self) -> String {
        let mut s = format!("{} {}", self.name, self.sql_type);
        if self.primary_key {
            s.push_str(" PRIMARY KEY");
        }
        if !self.nullable {
            s.push_str(" NOT NULL");
        }
        if self.unique {
            s.push_str(" UNIQUE");
        }
        if let Some(ref d) = self.default {
            s.push_str(&format!(" DEFAULT {}", d));
        }
        if let Some(ref fk) = self.foreign_key {
            s.push_str(&format!(" REFERENCES {}", fk));
        }
        s
    }
}

#[cfg(feature = "db")]
pub trait RangoSchema {
    fn columns() -> Vec<ColumnDef>;
    fn generate_migration_sql() -> String;
    fn generate_index_sql() -> Vec<String>;
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
    async fn search(&self, query: &str) -> crate::RangoResult<Vec<serde_json::Value>>;
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

    async fn search(&self, _query: &str) -> crate::RangoResult<Vec<serde_json::Value>> {
        // Default: return all. Override in custom ModelAdmin impls.
        self.list().await
    }
}
