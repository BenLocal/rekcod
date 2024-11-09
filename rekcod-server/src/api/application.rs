use axum::{
    body::Body,
    extract::Path,
    response::{IntoResponse as _, Response},
    Json,
};
use hyper::Request;
use rekcod_core::{
    api::{
        req::RenderTmplRequest,
        resp::{ApiJsonResponse, ApplicationResponse, RenderTmplResponse},
    },
    http::ApiError,
};
use tower::ServiceExt;

use crate::app::{engine::render_dynamic_tmpl, manager::get_app_manager};

pub async fn get_app_list() -> Result<Json<ApiJsonResponse<Vec<ApplicationResponse>>>, ApiError> {
    let apps = get_app_manager()
        .get_app_list()
        .await
        .into_iter()
        .filter_map(|app| {
            let info = &app.info.clone()?;
            Some(ApplicationResponse {
                name: info.name.clone(),
                description: info.description.clone(),
                tmpls: app.tmpls.clone(),
                id: app.id.clone(),
                version: info.version.clone(),
                values: info
                    .values
                    .as_ref()
                    .map(|x| serde_yaml::to_string(x).unwrap_or_default())
                    .unwrap_or_default(),
            })
        })
        .collect();
    Ok(ApiJsonResponse::success(apps).into())
}

#[axum::debug_handler]
pub async fn get_app_template_by_name(
    Path((name, tmpl)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let app_manager = get_app_manager();
    let app = match app_manager.get_app(&name).await {
        Some(app) => app,
        None => return Err(anyhow::anyhow!("App not found").into()),
    };
    let tmpl = match app.tmpls.iter().find(|t| t == &&tmpl) {
        Some(t) => t,
        None => return Err(anyhow::anyhow!("Tmpl not found").into()),
    };

    let resp = app
        .tmpl_service
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/{}", tmpl))
                .body(Body::empty())?,
        )
        .await?;

    Ok(resp.into_response())
}

pub async fn render_tmpl(
    Json(req): Json<RenderTmplRequest>,
) -> Result<Json<ApiJsonResponse<RenderTmplResponse>>, ApiError> {
    let ctx: serde_yaml::Value = serde_yaml::from_str(&req.tmpl_values)?;
    let content = render_dynamic_tmpl(&req.tmpl_context, ctx)?;
    Ok(ApiJsonResponse::success(RenderTmplResponse { content: content }).into())
}
