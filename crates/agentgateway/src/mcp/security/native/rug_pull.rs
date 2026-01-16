// Rug Pull Detection
//
// Monitors tool availability and integrity over time to detect sudden changes
// that could indicate a malicious server is "pulling the rug" by removing or
// modifying critical tools.
//
// Attack scenarios detected:
// - Tool removal: Server removes tools after initial trust is established
// - Schema changes: Server modifies tool input schemas to alter behavior
// - Description changes: Server modifies tool descriptions (potential prompt injection)
// - Tool additions: Server adds new tools (lower risk but tracked)
//
// The guard maintains an in-memory baseline per server and compares subsequent
// tools/list responses against it, calculating a risk score based on changes.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::time::Instant;

use super::NativeGuard;
use crate::mcp::security::{DenyReason, GuardContext, GuardDecision, GuardResult};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for Rug Pull Detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RugPullConfig {
    /// Enable baseline tracking
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Risk threshold for blocking (cumulative score triggers Deny)
    #[serde(default = "default_risk_threshold")]
    pub risk_threshold: u32,

    /// Risk weight for tool removal (default: 3 - high risk)
    #[serde(default = "default_removal_weight")]
    pub removal_weight: u32,

    /// Risk weight for schema changes (default: 3 - high risk)
    #[serde(default = "default_schema_change_weight")]
    pub schema_change_weight: u32,

    /// Risk weight for description changes (default: 2 - medium risk)
    #[serde(default = "default_description_change_weight")]
    pub description_change_weight: u32,

    /// Risk weight for tool additions (default: 1 - low risk)
    #[serde(default = "default_addition_weight")]
    pub addition_weight: u32,

    /// Enable/disable specific change type detection
    #[serde(default)]
    pub detect_changes: ChangeDetectionConfig,

    /// Whether to update baseline after allowing changes below threshold
    #[serde(default = "default_update_baseline_on_allow")]
    pub update_baseline_on_allow: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_risk_threshold() -> u32 {
    5
}

fn default_removal_weight() -> u32 {
    3
}

fn default_schema_change_weight() -> u32 {
    3
}

fn default_description_change_weight() -> u32 {
    2
}

fn default_addition_weight() -> u32 {
    1
}

fn default_update_baseline_on_allow() -> bool {
    true
}

fn default_true() -> bool {
    true
}

impl Default for RugPullConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            risk_threshold: default_risk_threshold(),
            removal_weight: default_removal_weight(),
            schema_change_weight: default_schema_change_weight(),
            description_change_weight: default_description_change_weight(),
            addition_weight: default_addition_weight(),
            detect_changes: ChangeDetectionConfig::default(),
            update_baseline_on_allow: default_update_baseline_on_allow(),
        }
    }
}

/// Fine-grained control over which change types to detect
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ChangeDetectionConfig {
    /// Detect tool removals (default: true)
    #[serde(default = "default_true")]
    pub removals: bool,

    /// Detect tool additions (default: true)
    #[serde(default = "default_true")]
    pub additions: bool,

    /// Detect description changes (default: true)
    #[serde(default = "default_true")]
    pub description_changes: bool,

    /// Detect schema changes (default: true)
    #[serde(default = "default_true")]
    pub schema_changes: bool,
}

impl Default for ChangeDetectionConfig {
    fn default() -> Self {
        Self {
            removals: default_true(),
            additions: default_true(),
            description_changes: default_true(),
            schema_changes: default_true(),
        }
    }
}

// ============================================================================
// Internal Data Structures
// ============================================================================

/// Unique fingerprint of a tool for efficient comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ToolFingerprint {
    /// Tool name (primary identifier)
    name: String,
    /// Hash of description (None if no description)
    description_hash: Option<u64>,
    /// Hash of serialized input_schema
    schema_hash: u64,
}

impl ToolFingerprint {
    /// Create fingerprint from an rmcp Tool
    fn from_tool(tool: &rmcp::model::Tool) -> Self {
        // Hash description if present
        let description_hash = tool.description.as_ref().map(|desc| {
            let mut hasher = DefaultHasher::new();
            desc.as_ref().hash(&mut hasher);
            hasher.finish()
        });

        // Hash serialized schema
        let schema_hash = {
            let mut hasher = DefaultHasher::new();
            // Serialize to JSON for consistent hashing
            if let Ok(json) = serde_json::to_string(&*tool.input_schema) {
                json.hash(&mut hasher);
            }
            hasher.finish()
        };

        Self {
            name: tool.name.to_string(),
            description_hash,
            schema_hash,
        }
    }
}

/// Baseline state for a single MCP server
#[derive(Debug, Clone)]
struct ServerBaseline {
    /// When the baseline was established (kept for potential future metrics/debugging)
    #[allow(dead_code)]
    established_at: Instant,
    /// Map of tool name -> fingerprint
    tools: HashMap<String, ToolFingerprint>,
    /// Number of times this baseline has been updated
    update_count: u64,
    /// Whether this server is blocked due to rug pull detection
    blocked: bool,
    /// Details of the block (for deny messages)
    block_reason: Option<String>,
}

impl ServerBaseline {
    /// Create initial baseline from tools list
    fn establish(tools: &[rmcp::model::Tool]) -> Self {
        let tools_map: HashMap<String, ToolFingerprint> = tools
            .iter()
            .map(|tool| {
                let fingerprint = ToolFingerprint::from_tool(tool);
                (tool.name.to_string(), fingerprint)
            })
            .collect();

        Self {
            established_at: Instant::now(),
            tools: tools_map,
            update_count: 0,
            blocked: false,
            block_reason: None,
        }
    }

    /// Mark this server as blocked due to rug pull detection
    fn block(&mut self, reason: String) {
        self.blocked = true;
        self.block_reason = Some(reason);
    }

    /// Compare current tools against baseline, return detected changes
    fn detect_changes(
        &self,
        current_tools: &[rmcp::model::Tool],
        config: &ChangeDetectionConfig,
    ) -> Vec<ToolChange> {
        let mut changes = Vec::new();
        let current_map: HashMap<String, ToolFingerprint> = current_tools
            .iter()
            .map(|t| (t.name.to_string(), ToolFingerprint::from_tool(t)))
            .collect();

        // Check for removals and modifications
        for (name, baseline_fp) in &self.tools {
            match current_map.get(name) {
                None => {
                    // Tool was removed
                    if config.removals {
                        changes.push(ToolChange::Removed {
                            name: name.clone(),
                        });
                    }
                }
                Some(current_fp) => {
                    // Check for modifications
                    if config.description_changes
                        && baseline_fp.description_hash != current_fp.description_hash
                    {
                        changes.push(ToolChange::DescriptionChanged {
                            name: name.clone(),
                            old_hash: baseline_fp.description_hash,
                            new_hash: current_fp.description_hash,
                        });
                    }
                    if config.schema_changes && baseline_fp.schema_hash != current_fp.schema_hash {
                        changes.push(ToolChange::SchemaChanged {
                            name: name.clone(),
                            old_hash: baseline_fp.schema_hash,
                            new_hash: current_fp.schema_hash,
                        });
                    }
                }
            }
        }

        // Check for additions
        if config.additions {
            for name in current_map.keys() {
                if !self.tools.contains_key(name) {
                    changes.push(ToolChange::Added { name: name.clone() });
                }
            }
        }

        changes
    }

    /// Update baseline with new tools
    fn update(&mut self, tools: &[rmcp::model::Tool]) {
        self.tools = tools
            .iter()
            .map(|tool| {
                let fingerprint = ToolFingerprint::from_tool(tool);
                (tool.name.to_string(), fingerprint)
            })
            .collect();
        self.update_count += 1;
    }
}

/// Types of changes detected between baseline and current tools
#[derive(Debug, Clone)]
enum ToolChange {
    /// Tool was present in baseline but removed
    Removed { name: String },
    /// Tool was added (not in baseline)
    Added { name: String },
    /// Tool description changed
    DescriptionChanged {
        name: String,
        #[allow(dead_code)]
        old_hash: Option<u64>,
        #[allow(dead_code)]
        new_hash: Option<u64>,
    },
    /// Tool schema changed
    SchemaChanged {
        name: String,
        #[allow(dead_code)]
        old_hash: u64,
        #[allow(dead_code)]
        new_hash: u64,
    },
}

impl ToolChange {
    fn change_type(&self) -> &'static str {
        match self {
            ToolChange::Removed { .. } => "removed",
            ToolChange::Added { .. } => "added",
            ToolChange::DescriptionChanged { .. } => "description_changed",
            ToolChange::SchemaChanged { .. } => "schema_changed",
        }
    }

    fn tool_name(&self) -> &str {
        match self {
            ToolChange::Removed { name }
            | ToolChange::Added { name }
            | ToolChange::DescriptionChanged { name, .. }
            | ToolChange::SchemaChanged { name, .. } => name,
        }
    }
}

// ============================================================================
// Detector Implementation
// ============================================================================

/// Rug Pull Detector implementation
pub struct RugPullDetector {
    config: RugPullConfig,
    /// Thread-safe storage: server_name -> baseline
    baselines: RwLock<HashMap<String, ServerBaseline>>,
}

impl RugPullDetector {
    pub fn new(config: RugPullConfig) -> Self {
        Self {
            config,
            baselines: RwLock::new(HashMap::new()),
        }
    }

    /// Calculate total risk score from detected changes
    fn calculate_risk_score(&self, changes: &[ToolChange]) -> u32 {
        changes
            .iter()
            .map(|change| match change {
                ToolChange::Removed { .. } => self.config.removal_weight,
                ToolChange::Added { .. } => self.config.addition_weight,
                ToolChange::DescriptionChanged { .. } => self.config.description_change_weight,
                ToolChange::SchemaChanged { .. } => self.config.schema_change_weight,
            })
            .sum()
    }

    /// Build detailed JSON for DenyReason
    fn build_change_details(&self, changes: &[ToolChange], risk_score: u32) -> serde_json::Value {
        let change_details: Vec<serde_json::Value> = changes
            .iter()
            .map(|change| {
                let weight = match change {
                    ToolChange::Removed { .. } => self.config.removal_weight,
                    ToolChange::Added { .. } => self.config.addition_weight,
                    ToolChange::DescriptionChanged { .. } => self.config.description_change_weight,
                    ToolChange::SchemaChanged { .. } => self.config.schema_change_weight,
                };
                serde_json::json!({
                    "type": change.change_type(),
                    "tool": change.tool_name(),
                    "weight": weight
                })
            })
            .collect();

        serde_json::json!({
            "changes": change_details,
            "total_risk_score": risk_score,
            "threshold": self.config.risk_threshold
        })
    }
}

impl NativeGuard for RugPullDetector {
    fn evaluate_tools_list(
        &self,
        tools: &[rmcp::model::Tool],
        context: &GuardContext,
    ) -> GuardResult {
        if !self.config.enabled {
            tracing::debug!("RugPullDetector disabled, allowing");
            return Ok(GuardDecision::Allow);
        }

        let server_name = &context.server_name;

        // Try to get existing baseline (read lock)
        {
            let baselines = self.baselines.read().expect("baselines lock poisoned");
            if let Some(baseline) = baselines.get(server_name) {
                // Check if already blocked
                if baseline.blocked {
                    tracing::warn!(
                        server = %server_name,
                        "Server is blocked due to previous rug pull detection"
                    );
                    return Ok(GuardDecision::Deny(DenyReason {
                        code: "rug_pull_server_blocked".to_string(),
                        message: format!(
                            "Server '{}' is blocked due to previous rug pull detection",
                            server_name
                        ),
                        details: baseline.block_reason.as_ref().map(|r| serde_json::json!({
                            "original_reason": r
                        })),
                    }));
                }

                // Compare against baseline
                let changes = baseline.detect_changes(tools, &self.config.detect_changes);

                if changes.is_empty() {
                    tracing::debug!(
                        server = %server_name,
                        tool_count = tools.len(),
                        "No changes detected from baseline"
                    );
                    return Ok(GuardDecision::Allow);
                }

                let risk_score = self.calculate_risk_score(&changes);

                tracing::info!(
                    server = %server_name,
                    change_count = changes.len(),
                    risk_score = risk_score,
                    threshold = self.config.risk_threshold,
                    "Tool changes detected"
                );

                // Log individual changes
                for change in &changes {
                    tracing::info!(
                        server = %server_name,
                        change_type = change.change_type(),
                        tool = change.tool_name(),
                        "Detected tool change"
                    );
                }

                if risk_score >= self.config.risk_threshold {
                    // Block the server and deny
                    let deny_message = format!(
                        "Suspicious tool changes detected (risk score: {} >= threshold: {})",
                        risk_score, self.config.risk_threshold
                    );
                    let details = self.build_change_details(&changes, risk_score);

                    // Upgrade to write lock to block the server
                    drop(baselines);
                    let mut baselines = self.baselines.write().expect("baselines lock poisoned");
                    if let Some(baseline) = baselines.get_mut(server_name) {
                        baseline.block(deny_message.clone());
                        tracing::warn!(
                            server = %server_name,
                            "Server blocked due to rug pull detection"
                        );
                    }

                    return Ok(GuardDecision::Deny(DenyReason {
                        code: "rug_pull_detected".to_string(),
                        message: deny_message,
                        details: Some(details),
                    }));
                }

                // Risk below threshold - optionally update baseline
                if self.config.update_baseline_on_allow {
                    // Need to release read lock and acquire write lock
                    drop(baselines);
                    let mut baselines = self.baselines.write().expect("baselines lock poisoned");
                    if let Some(baseline) = baselines.get_mut(server_name) {
                        baseline.update(tools);
                        tracing::debug!(
                            server = %server_name,
                            update_count = baseline.update_count,
                            "Baseline updated after low-risk changes"
                        );
                    }
                }

                return Ok(GuardDecision::Allow);
            }
        }

        // No baseline exists - establish one (first encounter)
        let mut baselines = self.baselines.write().expect("baselines lock poisoned");
        let baseline = ServerBaseline::establish(tools);

        tracing::info!(
            server = %server_name,
            tool_count = tools.len(),
            tools = ?tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>(),
            "Established initial baseline for server"
        );

        baselines.insert(server_name.clone(), baseline);

        Ok(GuardDecision::Allow)
    }

    fn evaluate_tool_invoke(
        &self,
        tool_name: &str,
        _arguments: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        if !self.config.enabled {
            return Ok(GuardDecision::Allow);
        }

        let server_name = &context.server_name;

        // Check if server is blocked
        let baselines = self.baselines.read().expect("baselines lock poisoned");
        if let Some(baseline) = baselines.get(server_name) {
            if baseline.blocked {
                tracing::warn!(
                    server = %server_name,
                    tool = %tool_name,
                    "Blocking tool invocation - server blocked due to rug pull detection"
                );
                return Ok(GuardDecision::Deny(DenyReason {
                    code: "rug_pull_server_blocked".to_string(),
                    message: format!(
                        "Tool '{}' blocked - server '{}' is blocked due to rug pull detection",
                        tool_name, server_name
                    ),
                    details: baseline.block_reason.as_ref().map(|r| serde_json::json!({
                        "original_reason": r,
                        "blocked_tool": tool_name
                    })),
                }));
            }
        }

        Ok(GuardDecision::Allow)
    }

    fn reset_server(&self, server_name: &str) {
        let mut baselines = self.baselines.write().expect("baselines lock poisoned");
        if baselines.remove(server_name).is_some() {
            tracing::info!(
                server = %server_name,
                "Reset rug pull baseline for server (session re-initialization)"
            );
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use std::borrow::Cow;
    use std::sync::Arc;

    fn create_test_tool(name: &str, description: Option<&str>) -> Tool {
        Tool {
            name: Cow::Owned(name.to_string()),
            description: description.map(|s| Cow::Owned(s.to_string())),
            icons: None,
            title: None,
            meta: None,
            input_schema: Arc::new(
                serde_json::from_value(serde_json::json!({"type": "object"})).unwrap(),
            ),
            annotations: None,
            output_schema: None,
        }
    }

    fn create_tool_with_schema(name: &str, schema: serde_json::Value) -> Tool {
        Tool {
            name: Cow::Owned(name.to_string()),
            description: Some(Cow::Owned("A tool".to_string())),
            icons: None,
            title: None,
            meta: None,
            input_schema: Arc::new(serde_json::from_value(schema).unwrap()),
            annotations: None,
            output_schema: None,
        }
    }

    fn create_test_context() -> GuardContext {
        GuardContext {
            server_name: "test-server".to_string(),
            identity: None,
            metadata: serde_json::json!({}),
        }
    }

    // ========== Basic Functionality Tests ==========

    #[test]
    fn test_first_encounter_establishes_baseline() {
        let config = RugPullConfig::default();
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let tools = vec![
            create_test_tool("tool1", Some("Description 1")),
            create_test_tool("tool2", Some("Description 2")),
        ];

        // First call should always Allow and establish baseline
        let result = detector.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Verify baseline was established
        let baselines = detector.baselines.read().unwrap();
        assert!(baselines.contains_key("test-server"));
        assert_eq!(baselines.get("test-server").unwrap().tools.len(), 2);
    }

    #[test]
    fn test_no_changes_allows() {
        let config = RugPullConfig::default();
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let tools = vec![
            create_test_tool("tool1", Some("Description 1")),
            create_test_tool("tool2", Some("Description 2")),
        ];

        // First call - establish baseline
        detector.evaluate_tools_list(&tools, &context).unwrap();

        // Second call with same tools - should Allow
        let result = detector.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_detects_tool_removal() {
        let config = RugPullConfig {
            risk_threshold: 5,
            removal_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Description 1")),
            create_test_tool("tool2", Some("Description 2")),
        ];

        // Establish baseline
        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove one tool (score = 3, below threshold of 5)
        let reduced_tools = vec![create_test_tool("tool1", Some("Description 1"))];

        let result = detector.evaluate_tools_list(&reduced_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Remove both tools from new baseline (score = 3 again, but cumulative changes)
        // Re-establish with both tools to test denial
        let detector2 = RugPullDetector::new(RugPullConfig {
            risk_threshold: 5,
            removal_weight: 3,
            ..Default::default()
        });
        detector2
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove both tools (score = 6, above threshold)
        let empty_tools: Vec<Tool> = vec![];
        let result = detector2.evaluate_tools_list(&empty_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_detects_tool_addition() {
        let config = RugPullConfig {
            risk_threshold: 5,
            addition_weight: 2, // Higher weight for testing
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![create_test_tool("tool1", Some("Description 1"))];

        // Establish baseline
        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Add tools (score = 2 per addition)
        let expanded_tools = vec![
            create_test_tool("tool1", Some("Description 1")),
            create_test_tool("tool2", Some("Description 2")),
            create_test_tool("tool3", Some("Description 3")),
        ];

        // 2 additions = 4, below threshold
        let result = detector.evaluate_tools_list(&expanded_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_detects_description_change() {
        let config = RugPullConfig {
            risk_threshold: 5,
            description_change_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Original description")),
            create_test_tool("tool2", Some("Another description")),
        ];

        // Establish baseline
        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Change description (score = 3, below threshold)
        let changed_tools = vec![
            create_test_tool("tool1", Some("Modified description")),
            create_test_tool("tool2", Some("Another description")),
        ];

        let result = detector.evaluate_tools_list(&changed_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_detects_schema_change() {
        let config = RugPullConfig {
            risk_threshold: 5,
            schema_change_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![create_tool_with_schema(
            "tool1",
            serde_json::json!({"type": "object", "properties": {"arg1": {"type": "string"}}}),
        )];

        // Establish baseline
        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Change schema (score = 3, below threshold)
        let changed_tools = vec![create_tool_with_schema(
            "tool1",
            serde_json::json!({"type": "object", "properties": {"arg1": {"type": "number"}}}),
        )];

        let result = detector.evaluate_tools_list(&changed_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    // ========== Risk Threshold Tests ==========

    #[test]
    fn test_below_threshold_allows() {
        let config = RugPullConfig {
            risk_threshold: 10,
            removal_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc 1")),
            create_test_tool("tool2", Some("Desc 2")),
            create_test_tool("tool3", Some("Desc 3")),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove 2 tools (score = 6, below threshold of 10)
        let reduced_tools = vec![create_test_tool("tool1", Some("Desc 1"))];

        let result = detector.evaluate_tools_list(&reduced_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_at_threshold_denies() {
        let config = RugPullConfig {
            risk_threshold: 6,
            removal_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc 1")),
            create_test_tool("tool2", Some("Desc 2")),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove both tools (score = 6, equals threshold)
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_above_threshold_denies() {
        let config = RugPullConfig {
            risk_threshold: 5,
            removal_weight: 3,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc 1")),
            create_test_tool("tool2", Some("Desc 2")),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove both tools (score = 6, above threshold)
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_cumulative_scoring() {
        let config = RugPullConfig {
            risk_threshold: 6,
            removal_weight: 2,
            schema_change_weight: 2,
            description_change_weight: 2,
            addition_weight: 1,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc 1")),
            create_tool_with_schema(
                "tool2",
                serde_json::json!({"type": "object", "properties": {}}),
            ),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove tool1 (2) + change tool2 schema (2) + add tool3 (1) = 5, below threshold
        let changed_tools = vec![
            create_tool_with_schema(
                "tool2",
                serde_json::json!({"type": "object", "properties": {"new": {"type": "string"}}}),
            ),
            create_test_tool("tool3", Some("New tool")),
        ];

        let result = detector.evaluate_tools_list(&changed_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    // ========== Configuration Tests ==========

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
enabled: true
risk_threshold: 10
removal_weight: 4
schema_change_weight: 4
description_change_weight: 2
addition_weight: 1
detect_changes:
  removals: true
  additions: false
  description_changes: true
  schema_changes: true
update_baseline_on_allow: false
"#;

        let config: RugPullConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.risk_threshold, 10);
        assert_eq!(config.removal_weight, 4);
        assert_eq!(config.schema_change_weight, 4);
        assert_eq!(config.description_change_weight, 2);
        assert_eq!(config.addition_weight, 1);
        assert!(config.detect_changes.removals);
        assert!(!config.detect_changes.additions);
        assert!(config.detect_changes.description_changes);
        assert!(config.detect_changes.schema_changes);
        assert!(!config.update_baseline_on_allow);
    }

    #[test]
    fn test_default_config() {
        let config = RugPullConfig::default();
        assert!(config.enabled);
        assert_eq!(config.risk_threshold, 5);
        assert_eq!(config.removal_weight, 3);
        assert_eq!(config.schema_change_weight, 3);
        assert_eq!(config.description_change_weight, 2);
        assert_eq!(config.addition_weight, 1);
        assert!(config.detect_changes.removals);
        assert!(config.detect_changes.additions);
        assert!(config.detect_changes.description_changes);
        assert!(config.detect_changes.schema_changes);
        assert!(config.update_baseline_on_allow);
    }

    #[test]
    fn test_custom_weights() {
        let config = RugPullConfig {
            risk_threshold: 10,
            removal_weight: 5, // Custom high weight
            addition_weight: 5, // Custom high weight
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![create_test_tool("tool1", Some("Desc"))];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove 1 tool (5) + add 1 tool (5) = 10, at threshold
        let changed_tools = vec![create_test_tool("tool2", Some("New tool"))];

        let result = detector.evaluate_tools_list(&changed_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_disable_specific_change_types() {
        let config = RugPullConfig {
            risk_threshold: 1, // Very low threshold
            removal_weight: 3,
            detect_changes: ChangeDetectionConfig {
                removals: false, // Disable removal detection
                ..Default::default()
            },
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![create_test_tool("tool1", Some("Desc"))];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove tool - but detection is disabled, should allow
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_disabled_guard_allows_all() {
        let config = RugPullConfig {
            enabled: false,
            risk_threshold: 0, // Would deny everything if enabled
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let tools = vec![create_test_tool("tool1", Some("Desc"))];

        // Should always allow when disabled
        let result = detector.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    // ========== Multi-Server Tests ==========

    #[test]
    fn test_separate_baselines_per_server() {
        let config = RugPullConfig::default();
        let detector = RugPullDetector::new(config);

        let context1 = GuardContext {
            server_name: "server-1".to_string(),
            identity: None,
            metadata: serde_json::json!({}),
        };

        let context2 = GuardContext {
            server_name: "server-2".to_string(),
            identity: None,
            metadata: serde_json::json!({}),
        };

        let tools1 = vec![create_test_tool("tool1", Some("Desc 1"))];
        let tools2 = vec![
            create_test_tool("tool2", Some("Desc 2")),
            create_test_tool("tool3", Some("Desc 3")),
        ];

        // Establish baselines for both servers
        detector.evaluate_tools_list(&tools1, &context1).unwrap();
        detector.evaluate_tools_list(&tools2, &context2).unwrap();

        // Verify separate baselines
        let baselines = detector.baselines.read().unwrap();
        assert_eq!(baselines.get("server-1").unwrap().tools.len(), 1);
        assert_eq!(baselines.get("server-2").unwrap().tools.len(), 2);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let config = RugPullConfig::default();
        let detector = Arc::new(RugPullDetector::new(config));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let detector = Arc::clone(&detector);
                thread::spawn(move || {
                    let context = GuardContext {
                        server_name: format!("server-{}", i),
                        identity: None,
                        metadata: serde_json::json!({}),
                    };
                    let tools = vec![create_test_tool(&format!("tool-{}", i), Some("Desc"))];
                    detector.evaluate_tools_list(&tools, &context)
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(matches!(result, Ok(GuardDecision::Allow)));
        }

        // All 10 servers should have baselines
        let baselines = detector.baselines.read().unwrap();
        assert_eq!(baselines.len(), 10);
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_empty_tools_list() {
        let config = RugPullConfig::default();
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        // Empty list should establish empty baseline
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Verify empty baseline
        let baselines = detector.baselines.read().unwrap();
        assert!(baselines.get("test-server").unwrap().tools.is_empty());
    }

    #[test]
    fn test_tools_without_description() {
        let config = RugPullConfig::default();
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let tools = vec![
            create_test_tool("tool1", None), // No description
            create_test_tool("tool2", Some("Has description")),
        ];

        // Should handle tools without description
        let result = detector.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Same tools again should allow
        let result = detector.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_baseline_update_on_allow() {
        let config = RugPullConfig {
            risk_threshold: 10, // High threshold
            removal_weight: 2,
            update_baseline_on_allow: true,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc")),
            create_test_tool("tool2", Some("Desc")),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove one tool (score = 2, below threshold)
        let reduced_tools = vec![create_test_tool("tool1", Some("Desc"))];

        let result = detector.evaluate_tools_list(&reduced_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Baseline should be updated - removing tool1 now should only score 2
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        // Score = 2 (removing tool1), below threshold of 10
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_no_baseline_update_when_disabled() {
        let config = RugPullConfig {
            risk_threshold: 10, // High threshold
            removal_weight: 2,
            update_baseline_on_allow: false, // Disabled
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![
            create_test_tool("tool1", Some("Desc")),
            create_test_tool("tool2", Some("Desc")),
        ];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        // Remove one tool (score = 2, below threshold)
        let reduced_tools = vec![create_test_tool("tool1", Some("Desc"))];

        let result = detector.evaluate_tools_list(&reduced_tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));

        // Baseline should NOT be updated - original baseline still has 2 tools
        // So removing tool1 should still compare against original (2 removals = 4)
        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);
        // Score = 4 (removing both tool1 and tool2 from original baseline)
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    // ========== Deny Reason Details Tests ==========

    #[test]
    fn test_deny_reason_contains_change_details() {
        let config = RugPullConfig {
            risk_threshold: 3,
            removal_weight: 4, // Will exceed threshold
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let initial_tools = vec![create_test_tool("critical_tool", Some("Important"))];

        detector
            .evaluate_tools_list(&initial_tools, &context)
            .unwrap();

        let empty_tools: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty_tools, &context);

        match result {
            Ok(GuardDecision::Deny(reason)) => {
                assert_eq!(reason.code, "rug_pull_detected");
                assert!(reason.message.contains("risk score"));
                assert!(reason.details.is_some());

                let details = reason.details.unwrap();
                assert!(details["changes"].is_array());
                assert_eq!(details["changes"].as_array().unwrap().len(), 1);
                assert_eq!(details["changes"][0]["type"], "removed");
                assert_eq!(details["changes"][0]["tool"], "critical_tool");
                assert_eq!(details["total_risk_score"], 4);
                assert_eq!(details["threshold"], 3);
            }
            other => panic!("Expected Deny decision, got {:?}", other),
        }
    }

    #[test]
    fn test_deny_reason_code() {
        let config = RugPullConfig {
            risk_threshold: 1,
            removal_weight: 2,
            ..Default::default()
        };
        let detector = RugPullDetector::new(config);
        let context = create_test_context();

        let tools = vec![create_test_tool("tool", Some("Desc"))];
        detector.evaluate_tools_list(&tools, &context).unwrap();

        let empty: Vec<Tool> = vec![];
        let result = detector.evaluate_tools_list(&empty, &context);

        match result {
            Ok(GuardDecision::Deny(reason)) => {
                assert_eq!(reason.code, "rug_pull_detected");
            }
            other => panic!("Expected Deny, got {:?}", other),
        }
    }

    // ========== Fingerprinting Tests ==========

    #[test]
    fn test_fingerprint_same_tool() {
        let tool1 = create_test_tool("test", Some("Description"));
        let tool2 = create_test_tool("test", Some("Description"));

        let fp1 = ToolFingerprint::from_tool(&tool1);
        let fp2 = ToolFingerprint::from_tool(&tool2);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_different_description() {
        let tool1 = create_test_tool("test", Some("Description 1"));
        let tool2 = create_test_tool("test", Some("Description 2"));

        let fp1 = ToolFingerprint::from_tool(&tool1);
        let fp2 = ToolFingerprint::from_tool(&tool2);

        assert_eq!(fp1.name, fp2.name);
        assert_ne!(fp1.description_hash, fp2.description_hash);
        assert_eq!(fp1.schema_hash, fp2.schema_hash);
    }

    #[test]
    fn test_fingerprint_different_schema() {
        let tool1 = create_tool_with_schema("test", serde_json::json!({"type": "object"}));
        let tool2 = create_tool_with_schema(
            "test",
            serde_json::json!({"type": "object", "properties": {}}),
        );

        let fp1 = ToolFingerprint::from_tool(&tool1);
        let fp2 = ToolFingerprint::from_tool(&tool2);

        assert_eq!(fp1.name, fp2.name);
        assert_ne!(fp1.schema_hash, fp2.schema_hash);
    }

    #[test]
    fn test_fingerprint_no_description() {
        let tool1 = create_test_tool("test", None);
        let tool2 = create_test_tool("test", Some("Has description"));

        let fp1 = ToolFingerprint::from_tool(&tool1);
        let fp2 = ToolFingerprint::from_tool(&tool2);

        assert!(fp1.description_hash.is_none());
        assert!(fp2.description_hash.is_some());
    }
}
