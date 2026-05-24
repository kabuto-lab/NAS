//! `with_tenant` — единственный entry-point для tenant-scoped DB queries.
//!
//! Спецификация: ENTITY.md §6 + ADR-001 D3.
//!
//! Wrapper открывает транзакцию, выполняет `SET LOCAL app.current_tenant_id = $1`,
//! делегирует closure'у `f(tx)`, commit'ит. RLS POLICY
//! `rls_cms_pages_tenant_isolation` срабатывает на каждом SELECT внутри транзакции.
//!
//! **CRITICAL:** требует PgBouncer в `transaction` pool mode. В `session` mode
//! SET LOCAL держится между запросами одного коннекта → pool exhaustion.

use ax_common::{AppError, TenantContext};
use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;

/// Execute a closure внутри транзакции с установленным tenant context.
///
/// # Errors
/// - `AppError::Database` если COMMIT, BEGIN или SET LOCAL fail
/// - Pass-through error из closure
///
/// # Example
/// ```no_run
/// # use sqlx::PgPool;
/// # use ax_common::{TenantContext, AppError};
/// # use ax_infrastructure::persistence::with_tenant;
/// # async fn example(pool: PgPool, ctx: TenantContext) -> Result<(), AppError> {
/// let count: i64 = with_tenant(&pool, &ctx, |tx| async move {
///     sqlx::query_scalar::<_, i64>("SELECT count(*) FROM cms_pages_v_active")
///         .fetch_one(&mut **tx)
///         .await
///         .map_err(|e| AppError::Database(e.to_string()))
/// })
/// .await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_tenant<T, F, Fut>(
    pool: &PgPool,
    ctx: &TenantContext,
    f: F,
) -> Result<T, AppError>
where
    F: for<'a> FnOnce(&'a mut Transaction<'_, Postgres>) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(format!("BEGIN failed: {e}")))?;

    sqlx::query("SET LOCAL app.current_tenant_id = $1")
        .bind(ctx.tenant_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(format!("SET LOCAL failed: {e}")))?;

    let result = f(&mut tx).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(format!("COMMIT failed: {e}")))?;

    Ok(result)
}
