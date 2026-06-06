#[cfg(feature = "db")]
use sqlx::Pool;
use std::sync::OnceLock;

#[cfg(feature = "db")]
static DB_POOL: OnceLock<Pool<sqlx::Any>> = OnceLock::new();

#[cfg(feature = "db")]
pub async fn init_db(database_url: &str) -> Result<(), sqlx::Error> {
    sqlx::any::install_default_drivers();
    let pool = sqlx::AnyPool::connect(database_url).await?;
    DB_POOL.set(pool).expect("DB pool already initialized");
    tracing::info!("🗄️  Rango DB connected : {}", database_url);
    Ok(())
}

#[cfg(feature = "db")]
pub fn db() -> &'static Pool<sqlx::Any> {
    DB_POOL.get().expect("DB non initialisée — appelle init_db() dans main()")
}

#[cfg(feature = "db")]
pub trait RangoModel: Sized + Send + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> {
    fn table_name() -> &'static str;

    async fn all() -> Result<Vec<Self>, sqlx::Error> {
        let query = format!("SELECT * FROM {}", Self::table_name());
        sqlx::query_as::<_, Self>(&query)
            .fetch_all(db())
            .await
    }

    async fn get_by_id(id: i64) -> Result<Option<Self>, sqlx::Error> {
        let query = format!("SELECT * FROM {} WHERE id = $1", Self::table_name());
        sqlx::query_as::<_, Self>(&query)
            .bind(id)
            .fetch_optional(db())
            .await
    }

    async fn delete_by_id(id: i64) -> Result<u64, sqlx::Error> {
        let query = format!("DELETE FROM {} WHERE id = $1", Self::table_name());
        let result = sqlx::query(&query)
            .bind(id)
            .execute(db())
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(feature = "db")]
pub async fn run_migrations(migrations_path: &str) -> Result<(), sqlx::Error> {
    use sqlx::migrate::Migrator;
    use std::path::Path;
    let migrator = Migrator::new(Path::new(migrations_path)).await?;
    migrator.run(db()).await?;
    tracing::info!("✅ Migrations applied from {}", migrations_path);
    Ok(())
}