export const DEFAULT_BACKEND_FORM = {
  name: "",
  weight: "1",
  // Route selection
  selectedBindPort: "",
  selectedListenerName: "",
  selectedRouteIndex: "",
  // Service backend fields
  serviceNamespace: "",
  serviceHostname: "",
  servicePort: "",
  // Host backend fields
  hostType: "address" as "address" | "hostname",
  hostAddress: "",
  hostHostname: "",
  hostPort: "",
  // MCP backend fields
  mcpTargets: [] as Array<{
    name: string;
    type: "sse" | "mcp" | "stdio" | "openapi";
    // SSE/MCP/OpenAPI fields
    host: string;
    port: string;
    path: string;
    // URL field for easier SSE/MCP/OpenAPI configuration
    fullUrl: string;
    // Stdio fields
    cmd: string;
    args: string[];
    env: Record<string, string>;
    // OpenAPI schema placeholder
    schema: boolean;
  }>,
  mcpStateful: true,
  // Security guards for MCP backends
  securityGuards: [] as Array<import("./types").SecurityGuard>,
  // AI backend fields
  aiProvider: "openAI" as "openAI" | "gemini" | "vertex" | "anthropic" | "bedrock" | "azureOpenAI",
  aiModel: "",
  aiRegion: "",
  aiProjectId: "",
  aiHostOverride: "",
  aiPathOverride: "",
  // for azure openai
  aiHost: "",
  aiApiVersion: "",
};

export const DEFAULT_MCP_TARGET = {
  name: "",
  type: "sse" as const,
  host: "",
  port: "",
  path: "",
  fullUrl: "",
  cmd: "",
  args: [] as string[],
  env: {} as Record<string, string>,
  schema: true,
};

export const BACKEND_TYPES = [
  { value: "mcp", label: "MCP", icon: "Target" },
  { value: "ai", label: "AI", icon: "Brain" },
  { value: "service", label: "Service", icon: "Cloud" },
  { value: "host", label: "Host", icon: "Server" },
  { value: "dynamic", label: "Dynamic", icon: "Globe" },
] as const;

export const BACKEND_TABLE_HEADERS = [
  "Name",
  "Type",
  "Listener",
  "Route",
  "Details",
  "Weight",
  "Actions",
] as const;

export const BACKEND_TYPE_COLORS = {
  mcp: "bg-primary hover:bg-primary/90",
  ai: "bg-green-500 hover:bg-green-600",
  service: "bg-orange-500 hover:bg-orange-600",
  host: "bg-red-500 hover:bg-red-600",
  dynamic: "bg-yellow-500 hover:bg-yellow-600",
  default: "bg-gray-500 hover:bg-gray-600",
} as const;

export const HOST_TYPES = [
  { value: "address", label: "Direct Address" },
  { value: "hostname", label: "Hostname + Port" },
] as const;

export const AI_MODEL_PLACEHOLDERS = {
  openAI: "gpt-4",
  gemini: "gemini-pro",
  vertex: "gemini-pro",
  anthropic: "claude-3-sonnet",
  bedrock: "anthropic.claude-3-sonnet",
  azureOpenAI: "gpt-4",
} as const;

export const AI_REGION_PLACEHOLDERS = {
  vertex: "us-central1",
  bedrock: "us-east-1",
} as const;

// =============================================================================
// Security Guards Constants
// =============================================================================

import type {
  SecurityGuardType,
  FailureMode,
  GuardPhase,
  PiiType,
  PiiAction,
  ScanField,
  ToolPoisoningGuard,
  RugPullGuard,
  ToolShadowingGuard,
  ServerWhitelistGuard,
  PiiGuard,
  WasmGuard,
  SecurityGuard,
} from "./types";

// Base security guard defaults (common fields)
export const DEFAULT_SECURITY_GUARD_BASE = {
  id: "",
  description: "",
  enabled: true,
  priority: 100,
  timeout_ms: 100,
  failure_mode: "fail_closed" as FailureMode,
  runs_on: ["response"] as GuardPhase[],
};

// Default Tool Poisoning guard
export const DEFAULT_TOOL_POISONING_GUARD: ToolPoisoningGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "tool_poisoning",
  strict_mode: true,
  custom_patterns: [],
  scan_fields: ["name", "description", "input_schema"],
  alert_threshold: 1,
};

// Default Rug Pull guard
export const DEFAULT_RUG_PULL_GUARD: RugPullGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "rug_pull",
  risk_threshold: 5,
};

// Default Tool Shadowing guard
export const DEFAULT_TOOL_SHADOWING_GUARD: ToolShadowingGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "tool_shadowing",
  block_duplicates: true,
  protected_names: [],
};

// Default Server Whitelist guard
export const DEFAULT_SERVER_WHITELIST_GUARD: ServerWhitelistGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "server_whitelist",
  allowed_servers: [],
  detect_typosquats: true,
  similarity_threshold: 0.85,
};

// Default PII guard
export const DEFAULT_PII_GUARD: PiiGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "pii",
  runs_on: ["request", "response", "tool_invoke"],
  detect: ["email", "credit_card"],
  action: "mask",
  min_score: 0.3,
  rejection_message: "",
};

// Default WASM guard
export const DEFAULT_WASM_GUARD: WasmGuard = {
  ...DEFAULT_SECURITY_GUARD_BASE,
  type: "wasm",
  runs_on: ["response", "tools_list"],
  module_path: "",
  max_memory_bytes: 10 * 1024 * 1024, // 10 MB
  max_fuel: 1_000_000, // 1 million instructions
  config: {},
};

// Security guard type metadata for UI
export const SECURITY_GUARD_TYPES: Array<{
  value: SecurityGuardType;
  label: string;
  description: string;
}> = [
  {
    value: "pii",
    label: "PII Detection",
    description: "Detect and mask personally identifiable information",
  },
  {
    value: "tool_poisoning",
    label: "Tool Poisoning",
    description: "Detect malicious patterns in tool descriptions",
  },
  {
    value: "rug_pull",
    label: "Rug Pull",
    description: "Monitor tool changes over time",
  },
  {
    value: "tool_shadowing",
    label: "Tool Shadowing",
    description: "Prevent duplicate or conflicting tools",
  },
  {
    value: "server_whitelist",
    label: "Server Whitelist",
    description: "Only allow trusted MCP servers",
  },
  {
    value: "wasm",
    label: "WASM Guard",
    description: "Custom WebAssembly security guard module",
  },
];

// Guard phases for UI selection
export const GUARD_PHASES: Array<{ value: GuardPhase; label: string }> = [
  { value: "request", label: "Request" },
  { value: "response", label: "Response" },
  { value: "tools_list", label: "Tools List" },
  { value: "tool_invoke", label: "Tool Invoke" },
];

// Failure modes for UI selection
export const FAILURE_MODES: Array<{ value: FailureMode; label: string; description: string }> = [
  { value: "fail_closed", label: "Fail Closed", description: "Block on failure (secure)" },
  { value: "fail_open", label: "Fail Open", description: "Allow on failure (available)" },
];

// PII types for UI selection
export const PII_TYPES: Array<{ value: PiiType; label: string }> = [
  { value: "email", label: "Email Address" },
  { value: "phone_number", label: "Phone Number" },
  { value: "ssn", label: "SSN (US)" },
  { value: "credit_card", label: "Credit Card" },
  { value: "ca_sin", label: "SIN (Canada)" },
  { value: "url", label: "URL" },
];

// PII actions for UI selection
export const PII_ACTIONS: Array<{ value: PiiAction; label: string; description: string }> = [
  { value: "mask", label: "Mask", description: "Replace PII with placeholders" },
  { value: "reject", label: "Reject", description: "Block requests containing PII" },
];

// Scan fields for tool poisoning
export const SCAN_FIELDS: Array<{ value: ScanField; label: string }> = [
  { value: "name", label: "Tool Name" },
  { value: "description", label: "Description" },
  { value: "input_schema", label: "Input Schema" },
];

// Helper to get default guard by type
export function getDefaultGuard(type: SecurityGuardType): SecurityGuard {
  const baseId = `${type}-${Date.now()}`;
  switch (type) {
    case "tool_poisoning":
      return { ...DEFAULT_TOOL_POISONING_GUARD, id: baseId };
    case "rug_pull":
      return { ...DEFAULT_RUG_PULL_GUARD, id: baseId };
    case "tool_shadowing":
      return { ...DEFAULT_TOOL_SHADOWING_GUARD, id: baseId };
    case "server_whitelist":
      return { ...DEFAULT_SERVER_WHITELIST_GUARD, id: baseId };
    case "pii":
      return { ...DEFAULT_PII_GUARD, id: baseId };
    case "wasm":
      return { ...DEFAULT_WASM_GUARD, id: baseId };
  }
}
