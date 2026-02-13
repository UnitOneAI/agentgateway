use std::borrow::Cow;
use std::sync::Arc;

use agent_core::trcng;
use futures_core::Stream;
use http::StatusCode;
use http::request::Parts;
use itertools::Itertools;
use opentelemetry::global::BoxedSpan;
use opentelemetry::trace::{SpanContext, SpanKind, TraceContextExt, TraceState};
use opentelemetry::{Context, TraceFlags};
use rmcp::ErrorData;
use rmcp::model::{
	ClientNotification, ClientRequest, Implementation, JsonRpcNotification, JsonRpcRequest,
	ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, Prompt,
	PromptsCapability, ProtocolVersion, RequestId, ResourcesCapability, ServerCapabilities,
	ServerInfo, ServerJsonRpcMessage, ServerResult, Tool, ToolsCapability,
};

use crate::http::Response;
use crate::http::jwt::Claims;
use crate::http::sessionpersistence::MCPSession;
use crate::mcp::mergestream::MergeFn;
use crate::mcp::rbac::{CelExecWrapper, Identity, McpAuthorizationSet};
use crate::mcp::router::McpBackendGroup;
use crate::mcp::streamablehttp::ServerSseMessage;
use crate::mcp::upstream::{IncomingRequestContext, UpstreamError};
use crate::mcp::{ClientError, MCPInfo, mergestream, rbac, upstream};
use crate::proxy::httpproxy::PolicyClient;
use crate::telemetry::log::AsyncLog;
use crate::telemetry::trc::TraceParent;

const DELIMITER: &str = "_";

fn resource_name(default_target_name: Option<&String>, target: &str, name: &str) -> String {
	if default_target_name.is_none() {
		format!("{target}{DELIMITER}{name}")
	} else {
		name.to_string()
	}
}

#[derive(Clone)]
pub struct Relay {
	upstreams: Arc<upstream::UpstreamGroup>,
	pub policies: McpAuthorizationSet,
	// If we have 1 target only, we don't prefix everything with 'target_'.
	// Else this is empty
	default_target_name: Option<String>,
	is_multiplexing: bool,
	security_guards: Arc<crate::mcp::security::GuardExecutor>,
}

impl std::fmt::Debug for Relay {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Relay")
			.field("policies", &self.policies)
			.field("default_target_name", &self.default_target_name)
			.field("is_multiplexing", &self.is_multiplexing)
			.finish()
	}
}

impl Relay {
	pub fn new(
		backend: McpBackendGroup,
		policies: McpAuthorizationSet,
		client: PolicyClient,
		guard_registry: crate::mcp::security::GuardExecutorRegistry,
	) -> anyhow::Result<Self> {
		let mut is_multiplexing = false;
		let default_target_name = if backend.targets.len() != 1 {
			is_multiplexing = true;
			None
		} else if backend.targets[0].always_use_prefix {
			None
		} else {
			Some(backend.targets[0].name.to_string())
		};

		// Get or create security guards from registry (enables hot-reload)
		let security_guards = guard_registry
			.get_or_create(&backend.name, backend.security_guards.clone())
			.unwrap_or_else(|e| {
				tracing::warn!("Failed to initialize security guards: {}", e);
				Arc::new(crate::mcp::security::GuardExecutor::empty())
			});

		Ok(Self {
			upstreams: Arc::new(upstream::UpstreamGroup::new(client, backend)?),
			policies,
			default_target_name,
			is_multiplexing,
			security_guards,
		})
	}

	pub fn parse_resource_name<'a, 'b: 'a>(
		&'a self,
		res: &'b str,
	) -> Result<(&'a str, &'b str), UpstreamError> {
		if let Some(default) = self.default_target_name.as_ref() {
			Ok((default.as_str(), res))
		} else {
			res
				.split_once(DELIMITER)
				.ok_or(UpstreamError::InvalidRequest(
					"invalid resource name".to_string(),
				))
		}
	}
}

impl Relay {
	pub fn get_sessions(&self) -> Option<Vec<MCPSession>> {
		let mut sessions = Vec::with_capacity(self.upstreams.size());
		for (_, us) in self.upstreams.iter_named() {
			sessions.push(us.get_session_state()?);
		}
		Some(sessions)
	}

	pub fn set_sessions(&self, sessions: Vec<MCPSession>) {
		for ((_, us), session) in self.upstreams.iter_named().zip(sessions) {
			us.set_session_id(&session.session, session.backend);
		}
	}
	pub fn count(&self) -> usize {
		self.upstreams.size()
	}

	pub fn is_multiplexing(&self) -> bool {
		self.is_multiplexing
	}
	pub fn default_target_name(&self) -> Option<String> {
		self.default_target_name.clone()
	}

	/// Evaluate security guards on a tool invocation
	pub fn evaluate_tool_invoke(
		&self,
		tool_name: &str,
		arguments: &serde_json::Value,
		server_name: &str,
		identity: Option<String>,
	) -> crate::mcp::security::GuardResult {
		let context = crate::mcp::security::GuardContext {
			server_name: server_name.to_string(),
			identity,
			metadata: serde_json::Value::Null,
		};
		self
			.security_guards
			.evaluate_tool_invoke(tool_name, arguments, &context)
	}

	/// Reset security guard state for all upstream servers (called on session re-initialization)
	pub fn reset_all_security_guards(&self) {
		for (name, _) in self.upstreams.iter_named() {
			self.security_guards.reset_server(&name);
		}
		tracing::info!("Reset security guard state for all upstream servers");
	}

	/// Fetch tools from all upstreams and establish security guard baselines.
	/// This is called after initialization to ensure baselines exist before any tools/call.
	/// Runs asynchronously and doesn't block the initialization response.
	pub async fn establish_security_baselines(&self, ctx: IncomingRequestContext) {
		use futures_util::StreamExt;

		tracing::info!("Establishing security guard baselines for all upstreams");

		for (server_name, upstream) in self.upstreams.iter_named() {
			// Evaluate connection phase guards (whitelist, typosquat detection)
			let context = crate::mcp::security::GuardContext {
				server_name: server_name.to_string(),
				identity: None,
				metadata: serde_json::Value::Null,
			};
			match self.security_guards.evaluate_connection(&server_name, None, &context) {
				Ok(crate::mcp::security::GuardDecision::Allow) => {
					tracing::info!(server = %server_name, "Connection guard: allowed");
				},
				Ok(crate::mcp::security::GuardDecision::Deny(reason)) => {
					tracing::warn!(
						server = %server_name,
						code = %reason.code,
						message = %reason.message,
						"Connection guard: BLOCKED server"
					);
					continue; // Skip this upstream entirely
				},
				Ok(_) => {},
				Err(e) => {
					tracing::error!(
						server = %server_name,
						error = %e,
						"Connection guard: error"
					);
					continue; // Skip on error (fail closed)
				},
			}

			// Create a tools/list request
			let request = JsonRpcRequest {
				jsonrpc: Default::default(),
				id: RequestId::Number(0),
				request: ClientRequest::ListToolsRequest(rmcp::model::ListToolsRequest {
					method: Default::default(),
					params: None,
					extensions: Default::default(),
				}),
			};

			// Send the request and collect tools
			match upstream.generic_stream(request, &ctx).await {
				Ok(stream) => {
					// Collect the response
					let messages: Vec<_> = stream.collect().await;
					for msg in messages {
						match msg {
							Ok(rmcp::model::ServerJsonRpcMessage::Response(resp)) => {
								if let rmcp::model::ServerResult::ListToolsResult(ltr) = resp.result {
									let tools = ltr.tools;
									tracing::info!(
										server = %server_name,
										tool_count = tools.len(),
										"Fetched tools for baseline establishment"
									);

									// Evaluate through guards to establish baseline
									let context = crate::mcp::security::GuardContext {
										server_name: server_name.to_string(),
										identity: None,
										metadata: serde_json::Value::Null,
									};

									match self.security_guards.evaluate_tools_list(&tools, &context) {
										Ok(crate::mcp::security::GuardDecision::Allow) => {
											tracing::info!(
												server = %server_name,
												"Baseline established successfully"
											);
										},
										Ok(crate::mcp::security::GuardDecision::Deny(reason)) => {
											tracing::warn!(
												server = %server_name,
												code = %reason.code,
												"Initial baseline denied (unexpected)"
											);
										},
										Ok(_) | Err(_) => {
											tracing::warn!(
												server = %server_name,
												"Baseline establishment had issues"
											);
										},
									}
								}
							},
							Ok(_) => {
								// Notifications or other messages, ignore
							},
							Err(e) => {
								tracing::warn!(
									server = %server_name,
									error = %e,
									"Error fetching tools for baseline"
								);
							},
						}
					}
				},
				Err(e) => {
					tracing::warn!(
						server = %server_name,
						error = %e,
						"Failed to fetch tools for baseline establishment"
					);
				},
			}
		}

		tracing::info!("Security guard baseline establishment complete");
	}

	pub fn merge_tools(&self, cel: CelExecWrapper) -> Box<MergeFn> {
		let policies = self.policies.clone();
		let default_target_name = self.default_target_name.clone();
		let security_guards = self.security_guards.clone();
		Box::new(move |streams| {
			let mut all_tools = Vec::new();

			// Process each server's tools individually for security guard evaluation
			for (server_name, s) in streams.into_iter() {
				let context = crate::mcp::security::GuardContext {
					server_name: server_name.to_string(),
					identity: None,
					metadata: serde_json::Value::Null,
				};

				let tools = match s {
					ServerResult::ListToolsResult(ltr) => ltr.tools,
					_ => vec![],
				};

				// Execute security guards on this server's tools list BEFORE merging
				// This ensures baselines are stored per-server, not under "merged"

				match security_guards.evaluate_tools_list(&tools, &context) {
					Ok(crate::mcp::security::GuardDecision::Allow) => {
						// Continue normally - add tools to merged list
					},
					Ok(crate::mcp::security::GuardDecision::Deny(reason)) => {
						tracing::error!(
							server = %server_name,
							code = %reason.code,
							message = %reason.message,
							"Security guard denied tools list for server"
						);
						return Err(crate::mcp::ClientError::new(anyhow::anyhow!(
							"Security guard denied for server '{}': {} - {}",
							server_name,
							reason.code,
							reason.message
						)));
					},
					Ok(crate::mcp::security::GuardDecision::Modify(_)) => {
						// TODO: Implement modification logic
						tracing::warn!(
							server = %server_name,
							"Security guard requested modification, but modification is not yet implemented"
						);
					},
					Err(e) => {
						tracing::error!(
							server = %server_name,
							error = %e,
							"Security guard execution failed"
						);
						return Err(crate::mcp::ClientError::new(anyhow::anyhow!(
							"Security guard failed for server '{}': {}",
							server_name,
							e
						)));
					},
				}

				// Apply authorization policies and rename for multiplexing
				let filtered_tools = tools
					.into_iter()
					.filter(|t| {
						policies.validate(
							&rbac::ResourceType::Tool(rbac::ResourceId::new(
								server_name.to_string(),
								t.name.to_string(),
							)),
							&cel,
						)
					})
					.map(|t| Tool {
						name: Cow::Owned(resource_name(
							default_target_name.as_ref(),
							server_name.as_str(),
							&t.name,
						)),
						..t
					})
					.collect_vec();

				all_tools.extend(filtered_tools);
			}

			Ok(
				ListToolsResult {
					tools: all_tools,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}

	pub fn merge_initialize(&self, pv: ProtocolVersion, multiplexing: bool) -> Box<MergeFn> {
		Box::new(move |s| {
			if !multiplexing {
				// Happy case: we can forward everything
				let (_, ServerResult::InitializeResult(ir)) = s.into_iter().next().unwrap() else {
					return Ok(Self::get_info(pv, multiplexing).into());
				};
				return Ok(ir.clone().into());
			}

			// Multiplexing is more complex. We need to find the lowest protocol version that all servers support.
			let lowest_version = s
				.into_iter()
				.flat_map(|(_, v)| match v {
					ServerResult::InitializeResult(r) => Some(r.protocol_version),
					_ => None,
				})
				.min_by_key(|i| i.to_string())
				.unwrap_or(pv);
			// For now, we just send our own info. In the future, we should merge the results from each upstream.
			Ok(Self::get_info(lowest_version, multiplexing).into())
		})
	}

	pub fn merge_prompts(&self, cel: CelExecWrapper) -> Box<MergeFn> {
		let policies = self.policies.clone();
		let default_target_name = self.default_target_name.clone();
		Box::new(move |streams| {
			let prompts = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let prompts = match s {
						ServerResult::ListPromptsResult(lpr) => lpr.prompts,
						_ => vec![],
					};
					prompts
						.into_iter()
						.filter(|p| {
							policies.validate(
								&rbac::ResourceType::Prompt(rbac::ResourceId::new(
									server_name.to_string(),
									p.name.to_string(),
								)),
								&cel,
							)
						})
						.map(|p| Prompt {
							name: resource_name(default_target_name.as_ref(), server_name.as_str(), &p.name),
							..p
						})
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListPromptsResult {
					prompts,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_resources(&self, cel: CelExecWrapper) -> Box<MergeFn> {
		let policies = self.policies.clone();
		Box::new(move |streams| {
			let resources = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let resources = match s {
						ServerResult::ListResourcesResult(lrr) => lrr.resources,
						_ => vec![],
					};
					resources
						.into_iter()
						.filter(|r| {
							policies.validate(
								&rbac::ResourceType::Resource(rbac::ResourceId::new(
									server_name.to_string(),
									r.uri.to_string(),
								)),
								&cel,
							)
						})
						// TODO(https://github.com/agentgateway/agentgateway/issues/404) map this to the service name,
						// if we add support for multiple services.
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListResourcesResult {
					resources,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_resource_templates(&self, cel: CelExecWrapper) -> Box<MergeFn> {
		let policies = self.policies.clone();
		Box::new(move |streams| {
			let resource_templates = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let resource_templates = match s {
						ServerResult::ListResourceTemplatesResult(lrr) => lrr.resource_templates,
						_ => vec![],
					};
					resource_templates
						.into_iter()
						.filter(|rt| {
							policies.validate(
								&rbac::ResourceType::Resource(rbac::ResourceId::new(
									server_name.to_string(),
									rt.uri_template.to_string(),
								)),
								&cel,
							)
						})
						// TODO(https://github.com/agentgateway/agentgateway/issues/404) map this to the service name,
						// if we add support for multiple services.
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListResourceTemplatesResult {
					resource_templates,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_empty(&self) -> Box<MergeFn> {
		Box::new(move |_| Ok(rmcp::model::ServerResult::empty(())))
	}
	pub async fn send_single(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		service_name: &str,
	) -> Result<Response, UpstreamError> {
		self
			.send_single_guarded(r, ctx, service_name, false, None)
			.await
	}

	/// Send a single request with optional response guard evaluation
	pub async fn send_single_guarded(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		service_name: &str,
		evaluate_response: bool,
		identity: Option<String>,
	) -> Result<Response, UpstreamError> {
		use futures_util::StreamExt;

		let id = r.id.clone();
		let Ok(us) = self.upstreams.get(service_name) else {
			return Err(UpstreamError::InvalidRequest(format!(
				"unknown service {service_name}"
			)));
		};
		let stream = us.generic_stream(r, &ctx).await?;

		if !evaluate_response {
			return messages_to_response(id, stream);
		}

		// Wrap the stream to evaluate responses through security guards
		let guards = self.security_guards.clone();
		let server_name = service_name.to_string();
		let identity_clone = identity.clone();
		let request_id = id.clone();

		let guarded_stream = stream.map(move |result| {
			match result {
				Ok(msg) => {
					// Try to evaluate the response through guards
					match evaluate_server_message(
						&msg,
						&guards,
						&server_name,
						identity_clone.clone(),
						request_id.clone(),
					) {
						Ok(modified_msg) => Ok(modified_msg),
						Err(e) => {
							tracing::warn!(error = %e, "Guard evaluation failed on response");
							// On guard error, return original message (fail-open for responses)
							Ok(msg)
						},
					}
				},
				Err(e) => Err(e),
			}
		});

		messages_to_response(id, guarded_stream)
	}

	// For some requests, we don't have a sane mapping of incoming requests to a specific
	// downstream service when multiplexing. Only forward when we have only one backend.
	pub async fn send_single_without_multiplexing(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let Some(service_name) = &self.default_target_name else {
			return Err(UpstreamError::InvalidMethod(r.request.method().to_string()));
		};
		self.send_single(r, ctx, service_name).await
	}
	pub async fn send_fanout_deletion(
		&self,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		for (_, con) in self.upstreams.iter_named() {
			con.delete(&ctx).await?;
		}
		Ok(accepted_response())
	}
	pub async fn send_fanout_get(
		&self,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((name, con.get_event_stream(&ctx).await?));
		}

		let ms = mergestream::MergeStream::new_without_merge(streams);
		messages_to_response(RequestId::Number(0), ms)
	}
	pub async fn send_fanout(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		merge: Box<MergeFn>,
	) -> Result<Response, UpstreamError> {
		let id = r.id.clone();
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((name, con.generic_stream(r.clone(), &ctx).await?));
		}

		let ms = mergestream::MergeStream::new(streams, id.clone(), merge);
		messages_to_response(id, ms)
	}
	pub async fn send_notification(
		&self,
		r: JsonRpcNotification<ClientNotification>,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((
				name,
				con
					.generic_notification(r.notification.clone(), &ctx)
					.await?,
			));
		}

		Ok(accepted_response())
	}
	fn get_info(pv: ProtocolVersion, multiplexing: bool) -> ServerInfo {
		let capabilities = if multiplexing {
			ServerCapabilities {
				completions: None,
				experimental: None,
				logging: None,
				tasks: None,
				tools: Some(ToolsCapability::default()),
				// These are not supported when multiplexing.
				prompts: None,
				resources: None,
			}
		} else {
			ServerCapabilities {
				completions: None,
				experimental: None,
				logging: None,
				tasks: None,
				tools: Some(ToolsCapability::default()),
				prompts: Some(PromptsCapability::default()),
				resources: Some(ResourcesCapability::default()),
			}
		};
		let instructions = Some(
			"This server is a gateway to a set of mcp servers. It is responsible for routing requests to the correct server and aggregating the results.".to_string(),
		);
		ServerInfo {
			protocol_version: pv,
			capabilities,
			server_info: Implementation::from_build_env(),
			instructions,
		}
	}
}

pub fn setup_request_log(
	http: Parts,
	span_name: &str,
) -> (BoxedSpan, AsyncLog<MCPInfo>, CelExecWrapper) {
	let traceparent = http.extensions.get::<TraceParent>();
	let mut ctx = Context::new();
	if let Some(tp) = traceparent {
		ctx = ctx.with_remote_span_context(SpanContext::new(
			tp.trace_id.into(),
			tp.span_id.into(),
			TraceFlags::new(tp.flags),
			true,
			TraceState::default(),
		));
	}
	let claims = http.extensions.get::<Claims>().cloned();

	let log = http
		.extensions
		.get::<AsyncLog<MCPInfo>>()
		.cloned()
		.unwrap_or_default();

	let cel = CelExecWrapper::new(http);

	let tracer = trcng::get_tracer();
	let _span = trcng::start_span(span_name.to_string(), &Identity::new(claims))
		.with_kind(SpanKind::Server)
		.start_with_context(tracer, &ctx);
	(_span, log, cel)
}

/// Evaluate a server message through security guards
fn evaluate_server_message(
	msg: &ServerJsonRpcMessage,
	guards: &crate::mcp::security::GuardExecutor,
	server_name: &str,
	identity: Option<String>,
	request_id: RequestId,
) -> Result<ServerJsonRpcMessage, String> {
	// Convert message to JSON for guard evaluation
	let json_value =
		serde_json::to_value(msg).map_err(|e| format!("Failed to serialize message: {}", e))?;

	let context = crate::mcp::security::GuardContext {
		server_name: server_name.to_string(),
		identity,
		metadata: serde_json::Value::Null,
	};

	// Evaluate through guards (using Response phase)
	match guards.evaluate_response(&json_value, &context) {
		Ok(crate::mcp::security::GuardDecision::Allow) => {
			// No modification needed
			Ok(msg.clone())
		},
		Ok(crate::mcp::security::GuardDecision::Deny(reason)) => {
			tracing::warn!(
				code = %reason.code,
				message = %reason.message,
				"Security guard denied response"
			);
			// Return an error message with the correct request ID
			Ok(ServerJsonRpcMessage::error(
				ErrorData::new(
					rmcp::model::ErrorCode(-32001),
					format!("Security guard denied: {}", reason.message),
					None,
				),
				request_id,
			))
		},
		Ok(crate::mcp::security::GuardDecision::Modify(
			crate::mcp::security::ModifyAction::Transform(modified_json),
		)) => {
			// Deserialize via string round-trip to work around serde limitation
			// with #[serde(flatten)] + #[serde(untagged)] combinations in rmcp types.
			// serde_json::from_value fails for these types, but from_str works correctly.
			// See: https://github.com/serde-rs/serde/issues/1183
			let json_string = serde_json::to_string(&modified_json)
				.map_err(|e| format!("Failed to serialize modified JSON: {}", e))?;
			match serde_json::from_str::<ServerJsonRpcMessage>(&json_string) {
				Ok(modified_msg) => {
					tracing::info!("Response modified by security guard");
					Ok(modified_msg)
				},
				Err(e) => {
					tracing::error!(
						error = %e,
						modified_json = %modified_json,
						"Failed to deserialize guard-modified response - returning ORIGINAL unmasked message. \
						 PII masking was NOT applied. Investigate serde compatibility."
					);
					Ok(msg.clone())
				},
			}
		},
		Ok(crate::mcp::security::GuardDecision::Modify(_)) => {
			// Other modify actions not supported
			Ok(msg.clone())
		},
		Err(e) => Err(format!("Guard evaluation error: {}", e)),
	}
}

fn messages_to_response(
	id: RequestId,
	stream: impl Stream<Item = Result<ServerJsonRpcMessage, ClientError>> + Send + 'static,
) -> Result<Response, UpstreamError> {
	use futures_util::StreamExt;
	use rmcp::model::ServerJsonRpcMessage;
	let stream = stream.map(move |rpc| {
		let r = match rpc {
			Ok(rpc) => rpc,
			Err(e) => {
				ServerJsonRpcMessage::error(ErrorData::internal_error(e.to_string(), None), id.clone())
			},
		};
		// TODO: is it ok to have no event_id here?
		ServerSseMessage {
			event_id: None,
			message: Arc::new(r),
		}
	});
	Ok(crate::mcp::session::sse_stream_response(stream, None))
}

fn accepted_response() -> Response {
	::http::Response::builder()
		.status(StatusCode::ACCEPTED)
		.body(crate::http::Body::empty())
		.expect("valid response")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::security::native::{PiiAction, PiiGuardConfig, PiiType};
	use crate::mcp::security::{
		FailureMode, GuardExecutor, GuardPhase, McpGuardKind, McpSecurityGuard,
	};

	fn create_pii_guard_executor(pii_types: Vec<PiiType>, action: PiiAction) -> GuardExecutor {
		let config = McpSecurityGuard {
			id: "test-pii".to_string(),
			description: None,
			priority: 50,
			failure_mode: FailureMode::FailClosed,
			timeout_ms: 100,
			runs_on: vec![GuardPhase::Response],
			enabled: true,
			kind: McpGuardKind::Pii(PiiGuardConfig {
				detect: pii_types,
				action,
				min_score: 0.3,
				rejection_message: None,
			}),
		};
		GuardExecutor::new(vec![config]).expect("Failed to create guard executor")
	}

	#[test]
	fn test_credit_card_masking_round_trip() {
		// Build a ServerJsonRpcMessage containing a credit card number
		// using from_str (the same way the gateway receives messages).
		let json_str = r#"{
			"jsonrpc": "2.0",
			"id": 1,
			"result": {
				"content": [
					{
						"type": "text",
						"text": "Your card number is 4111111111111111"
					}
				]
			}
		}"#;

		let msg: ServerJsonRpcMessage =
			serde_json::from_str(json_str).expect("Failed to parse test message");

		let guards = create_pii_guard_executor(vec![PiiType::CreditCard], PiiAction::Mask);

		let result = evaluate_server_message(&msg, &guards, "test-server", None, RequestId::Number(1));

		let modified = result.expect("evaluate_server_message should succeed");
		let modified_json =
			serde_json::to_value(&modified).expect("Failed to serialize modified message");

		let text = modified_json["result"]["content"][0]["text"]
			.as_str()
			.expect("Expected text content");

		assert!(
			text.contains("<CREDIT_CARD>"),
			"Credit card should be masked with <CREDIT_CARD>, got: {}",
			text
		);
		assert!(
			!text.contains("4111111111111111"),
			"Original credit card number should be removed, got: {}",
			text
		);
	}

	#[test]
	fn test_clean_message_passes_through() {
		let json_str = r#"{
			"jsonrpc": "2.0",
			"id": 1,
			"result": {
				"content": [
					{
						"type": "text",
						"text": "Hello, this is a clean message"
					}
				]
			}
		}"#;

		let msg: ServerJsonRpcMessage =
			serde_json::from_str(json_str).expect("Failed to parse test message");

		let guards = create_pii_guard_executor(vec![PiiType::CreditCard], PiiAction::Mask);

		let result = evaluate_server_message(&msg, &guards, "test-server", None, RequestId::Number(1));

		let returned = result.expect("Should succeed");
		let returned_json = serde_json::to_value(&returned).unwrap();
		let text = returned_json["result"]["content"][0]["text"]
			.as_str()
			.unwrap();
		assert_eq!(text, "Hello, this is a clean message");
	}
}
