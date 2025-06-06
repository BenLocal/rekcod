use axum::{
    body::Body,
    extract::Path,
    response::{IntoResponse as _, Response},
    Json,
};
use hyper::Request;
use rekcod_core::{
    api::{
        req::{AppDeployDeleteRequest, AppDeployRequest, RenderTmplRequest},
        resp::{ApiJsonResponse, ApplicationTmplResponse, RenderTmplResponse},
    },
    http::ApiError,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tower::ServiceExt;

use crate::{
    app::manager::get_app_tmpl_manager,
    app::{engine::render_dynamic_tmpl, manager::AppDeployInfo},
    db,
};

pub async fn get_app_tmpl_list(
) -> Result<Json<ApiJsonResponse<Vec<ApplicationTmplResponse>>>, ApiError> {
    let app_tmpls = get_app_tmpl_manager()
        .get_app_tmpl_list()
        .await
        .into_iter()
        .filter_map(|app| {
            let info = &app.info.clone()?;
            Some(ApplicationTmplResponse {
                name: info.name.clone(),
                description: info.description.clone(),
                tmpls: app.tmpls.clone(),
                id: app.id.clone(),
                version: info.version.clone(),
                qa: info.qa.clone(),
                values: None,
            })
        })
        .collect();
    Ok(ApiJsonResponse::success(app_tmpls).into())
}

pub async fn get_app_tmpl_by_id(
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<ApplicationTmplResponse>>, ApiError> {
    get_app_tmpl_manager()
        .get_app_tmpl(&id)
        .await
        .map(|app| {
            let info = &app.info.clone().unwrap();
            let resp = ApplicationTmplResponse {
                name: info.name.clone(),
                description: info.description.clone(),
                tmpls: app.tmpls.clone(),
                id: app.id.clone(),
                version: info.version.clone(),
                qa: info.qa.clone(),
                values: None,
            };
            ApiJsonResponse::success(ApplicationTmplResponse::from(resp)).into()
        })
        .ok_or(anyhow::anyhow!("App not found").into())
}

#[axum::debug_handler]
pub async fn get_app_template_by_name(
    Path((name, tmpl)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let app_manager = get_app_tmpl_manager();
    let app = match app_manager.get_app_tmpl(&name).await {
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

pub async fn dynamic_render_tmpl(
    Json(req): Json<RenderTmplRequest>,
) -> Result<Json<ApiJsonResponse<RenderTmplResponse>>, ApiError> {
    let ctx: serde_yaml::Value = serde_yaml::from_str(&req.tmpl_values)?;
    let content = render_dynamic_tmpl(&req.tmpl_context, ctx).await?;
    Ok(ApiJsonResponse::success(RenderTmplResponse { content: content }).into())
}

pub async fn list_deploy_app() -> Result<Json<ApiJsonResponse<Vec<AppDeployInfo>>>, ApiError> {
    let db = db::repository().await;

    let apps = db.kvs.select("app", None, None, None).await?;
    Ok(ApiJsonResponse::success(
        apps.iter()
            .filter_map(|x| {
                let v: AppDeployInfo = serde_json::from_str(&x.value).ok()?;
                Some(v)
            })
            .collect::<Vec<_>>(),
    )
    .into())
}

pub async fn delete_deploy_app(
    Json(req): Json<AppDeployDeleteRequest>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let db = db::repository().await;
    db.kvs
        .delete("app", Some(&req.app_name), None, None)
        .await?;
    Ok(ApiJsonResponse::success(()).into())
}

pub async fn app_deploy(Json(req): Json<AppDeployRequest>) -> Result<Response, ApiError> {
    let app_tmpl_manager = get_app_tmpl_manager();
    let app_tmpl = match app_tmpl_manager.get_app_tmpl(&req.app_name).await {
        Some(app) => app,
        None => return Err(anyhow::anyhow!("App not found").into()),
    };
    let (tx_chan, rx_chan) = tokio::sync::mpsc::unbounded_channel::<String>();
    crate::app::manager::deploy(&req, &app_tmpl, &tx_chan).await?;
    Ok(Response::new(Body::from_stream(
        UnboundedReceiverStream::new(rx_chan).map(|x| anyhow::Ok(x)),
    )))
}
