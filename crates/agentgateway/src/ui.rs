use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, Method};
use hyper::body::Incoming;
use include_dir::{Dir, include_dir};
use serde::{Serialize, Serializer};
use serde_json::Value;
use tower::ServiceExt;
use tower_http::cors::CorsLayer;
use tower_serve_static::ServeDir;

use crate::management::admin::{AdminFallback, AdminResponse};
use crate::mcp::security::McpGuardKind;
use crate::{Config, ConfigSource, client, yamlviajson};

pub struct UiHandler {
	router: Router,
}

#[derive(Clone, Debug)]
struct App {
	state: Arc<Config>,
	client: client::Client,
}

impl App {
	pub fn cfg(&self) -> Result<ConfigSource, ErrorResponse> {
		self
			.state
			.xds
			.local_config
			.clone()
			.ok_or(ErrorResponse::String("local config not setup".to_string()))
	}
}

lazy_static::lazy_static! {
	static ref ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../ui/out");
}

impl UiHandler {
	pub fn new(cfg: Arc<Config>) -> Self {
		let ui_service = ServeDir::new(&ASSETS_DIR);
		let router = Router::new()
			// Redirect to the UI
			.route("/config", get(get_config).post(write_config))
			.route("/api/v1/guards/schemas", get(get_guard_schemas))
			.nest_service("/ui", ui_service)
			.route("/", get(|| async { Redirect::permanent("/ui") }))
			.layer(add_cors_layer())
			.with_state(App {
				state: cfg.clone(),
				client: client::Client::new(&cfg.dns, None, Default::default(), None),
			});
		Self { router }
	}
}

#[derive(Debug, thiserror::Error)]
enum ErrorResponse {
	#[error("{0}")]
	String(String),
	#[error("{0}")]
	Anyhow(#[from] anyhow::Error),
}

impl Serialize for ErrorResponse {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl IntoResponse for ErrorResponse {
	fn into_response(self) -> Response {
		(StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
	}
}

async fn get_config(State(app): State<App>) -> Result<Json<Value>, ErrorResponse> {
	let s = app.cfg()?.read_to_string().await?;
	let v: Value = yamlviajson::from_str(&s).map_err(|e| ErrorResponse::Anyhow(e.into()))?;
	Ok(Json(v))
}

async fn write_config(
	State(app): State<App>,
	Json(config_json): Json<Value>,
) -> Result<Json<Value>, ErrorResponse> {
	let config_source = app.cfg()?;

	let file_path = match &config_source {
		ConfigSource::File(path) => path,
		ConfigSource::Static(_) => {
			return Err(ErrorResponse::String(
				"Cannot write to static config".to_string(),
			));
		},
	};
	let yaml_content =
		yamlviajson::to_string(&config_json).map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	if let Err(e) = crate::types::local::NormalizedLocalConfig::from(
		&app.state,
		app.client.clone(),
		app.state.gateway(),
		yaml_content.as_str(),
	)
	.await
	{
		return Err(ErrorResponse::String(e.to_string()));
	}

	// Write the YAML content to the file
	fs_err::tokio::write(file_path, yaml_content)
		.await
		.map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	// Return success response
	Ok(Json(
		serde_json::json!({"status": "success", "message": "Configuration written successfully"}),
	))
}

/// GET /api/v1/guards/schemas
/// Returns JSON Schemas for all guards (native schemas are embedded in the frontend;
/// WASM guard schemas are extracted by loading each WASM module and calling get-settings-schema).
async fn get_guard_schemas(State(app): State<App>) -> Result<Json<Value>, ErrorResponse> {
	let mut schemas = serde_json::Map::new();

	// Read config to find WASM guards
	if let Ok(cfg_source) = app.cfg() {
		if let Ok(yaml_str) = cfg_source.read_to_string().await {
			if let Ok(config_val) = yamlviajson::from_str::<Value>(&yaml_str) {
				collect_wasm_schemas_from_config(&config_val, &mut schemas);
			}
		}
	}

	Ok(Json(serde_json::json!({
		"schemas": schemas,
	})))
}

/// Walk the config JSON to find WASM guard entries and extract their schemas.
/// Returns schemas keyed by x-guard-meta.guardType (or guard id as fallback),
/// matching the GuardSchemasResponse format expected by the frontend.
#[allow(unused_variables)]
fn collect_wasm_schemas_from_config(
	config: &Value,
	schemas: &mut serde_json::Map<String, Value>,
) {
	// Navigate: backends[] -> mcp -> security_guards[]
	let Some(backends) = config.get("backends").and_then(|v| v.as_array()) else {
		return;
	};

	for backend in backends {
		let Some(mcp) = backend.get("mcp") else {
			continue;
		};
		let Some(guards) = mcp.get("security_guards").and_then(|v| v.as_array()) else {
			continue;
		};

		for guard_val in guards {
			let Some(guard_type) = guard_val.get("type").and_then(|v| v.as_str()) else {
				continue;
			};
			if guard_type != "wasm" {
				continue;
			}

			let Some(guard_id) = guard_val.get("id").and_then(|v| v.as_str()) else {
				continue;
			};

			// Try to deserialize as McpGuardKind to get WasmGuardConfig
			if let Ok(kind) = serde_json::from_value::<McpGuardKind>(guard_val.clone()) {
				#[cfg(feature = "wasm-guards")]
				if let McpGuardKind::Wasm(wasm_cfg) = kind {
					match crate::mcp::security::wasm::WasmGuard::new(
						guard_id.to_string(),
						wasm_cfg,
					) {
						Ok(wasm_guard) => {
							if let Ok(schema_str) = wasm_guard.get_settings_schema() {
								if let Ok(schema_val) =
									serde_json::from_str::<Value>(&schema_str)
								{
									// Use x-guard-meta.guardType as key, fall back to guard id
									let schema_key = schema_val
										.get("x-guard-meta")
										.and_then(|m| m.get("guardType"))
										.and_then(|v| v.as_str())
										.unwrap_or(guard_id)
										.to_string();

									schemas.insert(schema_key, schema_val);
								}
							}
						}
						Err(e) => {
							tracing::warn!(
								guard_id = guard_id,
								error = %e,
								"Failed to load WASM guard for schema extraction"
							);
						}
					}
				}

				#[cfg(not(feature = "wasm-guards"))]
				{
					let _ = kind;
					tracing::debug!(
						guard_id = guard_id,
						"WASM guards feature not enabled, skipping schema extraction"
					);
				}
			}
		}
	}
}

pub fn add_cors_layer() -> CorsLayer {
	CorsLayer::new()
		.allow_origin(
			[
				"http://0.0.0.0:3000",
				"http://localhost:3000",
				"http://127.0.0.1:3000",
				"http://0.0.0.0:19000",
				"http://127.0.0.1:19000",
				"http://localhost:19000",
			]
			.map(|origin| origin.parse::<HeaderValue>().unwrap()),
		)
		.allow_headers([
			CONTENT_TYPE,
			AUTHORIZATION,
			HeaderName::from_static("x-requested-with"),
		])
		.allow_methods([
			Method::GET,
			Method::POST,
			Method::PUT,
			Method::DELETE,
			Method::OPTIONS,
		])
		.allow_credentials(true)
		.expose_headers([CONTENT_TYPE, CONTENT_LENGTH])
		.max_age(Duration::from_secs(3600))
}

impl AdminFallback for UiHandler {
	fn handle(&self, req: http::Request<Incoming>) -> AdminResponse {
		let router = self.router.clone();
		Box::pin(async { router.oneshot(req).await.unwrap() })
	}
}
