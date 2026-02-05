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
  // Security guards for MCP backend (UnitOne Extension)
  securityGuards: [] as Array<{
    id: string;
    description?: string;
    priority?: number;
    failureMode?: "fail_closed" | "fail_open";
    timeoutMs?: number;
    runsOn?: Array<"request" | "response" | "tools_list" | "tool_invoke">;
    enabled?: boolean;
    type: "tool_poisoning" | "rug_pull" | "tool_shadowing" | "server_whitelist" | "pii" | "wasm";
    // Type-specific config
    strictMode?: boolean;
    customPatterns?: string[];
    alertThreshold?: number;
    scanFields?: string[];
    changeThreshold?: number;
    monitoredChangeTypes?: string[];
    updateBaseline?: boolean;
    shadowingPatterns?: string[];
    allowedServers?: string[];
    detect?: Array<"email" | "phone_number" | "ssn" | "credit_card" | "ca_sin" | "url">;
    action?: "mask" | "reject";
    minScore?: number;
    rejectionMessage?: string;
    modulePath?: string;
    maxMemory?: number;
    config?: Record<string, unknown>;
  }>,
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
