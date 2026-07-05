use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};

use crate::db::RangoAdminOps;
use crate::error::RangoError;
use crate::responses::redirect;

// ─── Environment ─────────────────────────────────────────────────────────────

static ADMIN_ENV: std::sync::OnceLock<minijinja::Environment<'static>> =
    std::sync::OnceLock::new();

fn get_admin_env() -> &'static minijinja::Environment<'static> {
    ADMIN_ENV.get_or_init(|| {
        let mut env = minijinja::Environment::new();
        // Ces unwrap sont acceptables : les templates sont embedded dans le binaire
        // via include_str!, donc un échec ici = bug de développement, pas runtime.
        env.add_template("admin_base.html", include_str!("templates/admin_base.html"))
            .unwrap();
        env.add_template(
            "admin_dashboard.html",
            include_str!("templates/admin_dashboard.html"),
        )
        .unwrap();
        env.add_template(
            "admin_model_list.html",
            include_str!("templates/admin_model_list.html"),
        )
        .unwrap();
        env.add_template(
            "admin_model_form.html",
            include_str!("templates/admin_model_form.html"),
        )
        .unwrap();
        env
    })
}

// Helper : rendu d'un template avec gestion d'erreur propre
fn render_admin(template_name: &str, ctx: serde_json::Value) -> Result<Html<String>, RangoError> {
    let env = get_admin_env();
    let tmpl = env
        .get_template(template_name)
        .map_err(|e| RangoError::TemplateNotFound(format!("{}: {}", template_name, e)))?;
    let html = tmpl
        .render(ctx)
        .map_err(|e| RangoError::RenderError(e.to_string()))?;
    Ok(Html(html))
}

fn get_sidebar_context(admin: &RangoAdmin) -> Vec<String> {
    admin.models.iter().map(|m| m.model_name().to_string()).collect()
}

// ─── RangoAdmin Site Registrar ───────────────────────────────────────────────

#[derive(Clone)]
pub struct RangoAdmin {
    pub models: Vec<Arc<dyn RangoAdminOps>>,
}

impl RangoAdmin {
    pub fn new() -> Self {
        RangoAdmin { models: Vec::new() }
    }

    pub fn register<T>(&mut self)
    where
        T: crate::db::RangoModel + crate::db::RangoAdminMetadata + Send + Sync + 'static,
    {
        self.models.push(Arc::new(crate::db::ModelAdmin::<T>::new()));
    }

    pub fn router(self) -> Router {
        let admin_state = Arc::new(self);
        Router::new()
            .route("/", get(admin_dashboard))
            .route("/:model_name", get(admin_model_list))
            .route(
                "/:model_name/add",
                get(admin_model_add).post(admin_model_add_submit),
            )
            .route(
                "/:model_name/:id",
                get(admin_model_edit).post(admin_model_edit_submit),
            )
            .route("/:model_name/:id/delete", post(admin_model_delete))
            .with_state(admin_state)
    }
}

impl Default for RangoAdmin {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Admin Handlers ──────────────────────────────────────────────────────────

async fn admin_dashboard(State(admin): State<Arc<RangoAdmin>>) -> impl IntoResponse {
    let mut model_summaries = Vec::new();
    for model in &admin.models {
        let count = model.list().await.map(|list| list.len()).unwrap_or(0);
        model_summaries.push(serde_json::json!({
            "name": model.model_name(),
            "count": count,
        }));
    }

    let sidebar = get_sidebar_context(&admin);
    match render_admin(
        "admin_dashboard.html",
        serde_json::json!({
            "sidebar_models": sidebar,
            "models": model_summaries,
        }),
    ) {
        Ok(html) => html.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_list(
    State(admin): State<Arc<RangoAdmin>>,
    Path(model_name): Path<String>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let items = match model.list().await {
        Ok(it) => it,
        Err(e) => return e.into_response(),
    };

    let fields = model.fields();
    let sidebar = get_sidebar_context(&admin);
    match render_admin(
        "admin_model_list.html",
        serde_json::json!({
            "sidebar_models": sidebar,
            "model_name": model.model_name(),
            "fields": fields,
            "items": items,
        }),
    ) {
        Ok(html) => html.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_add(
    State(admin): State<Arc<RangoAdmin>>,
    Path(model_name): Path<String>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let fields = model.fields();
    let sidebar = get_sidebar_context(&admin);
    match render_admin(
        "admin_model_form.html",
        serde_json::json!({
            "sidebar_models": sidebar,
            "model_name": model.model_name(),
            "fields": fields,
            "is_edit": false,
            "item": serde_json::Value::Null,
        }),
    ) {
        Ok(html) => html.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_add_submit(
    State(admin): State<Arc<RangoAdmin>>,
    Path(model_name): Path<String>,
    Form(form_data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match model.save(None, &form_data).await {
        Ok(_) => redirect(&format!("/admin/{}", model.model_name())).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_edit(
    State(admin): State<Arc<RangoAdmin>>,
    Path((model_name, id)): Path<(String, i64)>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let item = match model.get(id).await {
        Ok(Some(it)) => it,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return e.into_response(),
    };

    let fields = model.fields();
    let sidebar = get_sidebar_context(&admin);
    match render_admin(
        "admin_model_form.html",
        serde_json::json!({
            "sidebar_models": sidebar,
            "model_name": model.model_name(),
            "fields": fields,
            "is_edit": true,
            "item": item,
        }),
    ) {
        Ok(html) => html.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_edit_submit(
    State(admin): State<Arc<RangoAdmin>>,
    Path((model_name, id)): Path<(String, i64)>,
    Form(form_data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match model.save(Some(id), &form_data).await {
        Ok(_) => redirect(&format!("/admin/{}", model.model_name())).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn admin_model_delete(
    State(admin): State<Arc<RangoAdmin>>,
    Path((model_name, id)): Path<(String, i64)>,
) -> impl IntoResponse {
    let model = match admin
        .models
        .iter()
        .find(|m| m.model_name().eq_ignore_ascii_case(&model_name))
    {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match model.delete(id).await {
        Ok(_) => redirect(&format!("/admin/{}", model.model_name())).into_response(),
        Err(e) => e.into_response(),
    }
}
