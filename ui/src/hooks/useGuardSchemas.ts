/**
 * useGuardSchemas - Hook for fetching and managing guard schemas
 *
 * Provides access to JSON Schemas exported by guards for dynamic
 * form generation. Falls back to embedded schemas when API is unavailable.
 */

import { useState, useEffect, useCallback } from "react";
import {
  type GuardSettingsSchema,
  type GuardSchemasResponse,
  type GuardTypeSummary,
  resolveWithDefaults,
} from "@/lib/guard-schema-types";

// API URL - use relative for same-origin requests
const API_URL = "";

// =============================================================================
// Embedded Schemas (fallback when API unavailable)
// =============================================================================

/**
 * Native guard schemas embedded in the UI.
 * These match the Rust guard implementations.
 */
const NATIVE_GUARD_SCHEMAS: Record<string, GuardSettingsSchema> = {
  tool_poisoning: {
    $id: "agentgateway://guards/tool-poisoning/v1",
    title: "Tool Poisoning Detection",
    description: "Detects malicious patterns in MCP tool descriptions",
    type: "object",
    properties: {
      strict_mode: {
        type: "boolean",
        title: "Strict Mode",
        description: "Block on any suspicious pattern match",
        default: true,
        "x-ui": { order: 1 },
      },
      custom_patterns: {
        type: "array",
        title: "Custom Patterns",
        description: "Additional regex patterns to detect",
        items: { type: "string" },
        default: [],
        "x-ui": { component: "tags", placeholder: "(?i)custom_pattern", order: 2 },
      },
      scan_fields: {
        type: "array",
        title: "Scan Fields",
        description: "Which tool fields to scan for patterns",
        items: {
          type: "string",
          enum: ["name", "description", "input_schema"],
        },
        default: ["name", "description", "input_schema"],
        "x-ui": {
          component: "multiselect",
          labels: {
            name: "Tool Name",
            description: "Tool Description",
            input_schema: "Input Schema",
          },
          order: 3,
        },
      },
      alert_threshold: {
        type: "integer",
        title: "Alert Threshold",
        description: "Number of pattern matches to trigger alert",
        default: 1,
        minimum: 1,
        "x-ui": { order: 4 },
      },
    },
    "x-guard-meta": {
      guardType: "tool_poisoning",
      version: "1.0.0",
      category: "detection",
      defaultRunsOn: ["tools_list"],
      icon: "shield-alert",
    },
  },

  rug_pull: {
    $id: "agentgateway://guards/rug-pull/v1",
    title: "Rug Pull Detection",
    description: "Detects tools that change behavior after initial trust",
    type: "object",
    properties: {
      risk_threshold: {
        type: "number",
        title: "Risk Threshold",
        description: "Minimum risk score to trigger alert (0-1)",
        default: 0.7,
        minimum: 0,
        maximum: 1,
        "x-ui": { component: "slider", order: 1 },
      },
    },
    "x-guard-meta": {
      guardType: "rug_pull",
      version: "1.0.0",
      category: "detection",
      defaultRunsOn: ["tools_list"],
      icon: "alert-triangle",
    },
  },

  tool_shadowing: {
    $id: "agentgateway://guards/tool-shadowing/v1",
    title: "Tool Shadowing Detection",
    description: "Detects tools that shadow built-in or protected tool names",
    type: "object",
    properties: {
      block_duplicates: {
        type: "boolean",
        title: "Block Duplicates",
        description: "Block tools with duplicate names across servers",
        default: true,
        "x-ui": { order: 1 },
      },
      protected_names: {
        type: "array",
        title: "Protected Names",
        description: "Tool names that cannot be overridden",
        items: { type: "string" },
        default: [],
        "x-ui": { component: "tags", placeholder: "protected_tool", order: 2 },
      },
    },
    "x-guard-meta": {
      guardType: "tool_shadowing",
      version: "1.0.0",
      category: "detection",
      defaultRunsOn: ["tools_list"],
      icon: "copy",
    },
  },

  server_whitelist: {
    $id: "agentgateway://guards/server-whitelist/v1",
    title: "Server Whitelist",
    description: "Only allow connections to approved MCP servers",
    type: "object",
    properties: {
      allowed_servers: {
        type: "array",
        title: "Allowed Servers",
        description: "List of allowed server names or patterns",
        items: { type: "string" },
        default: [],
        "x-ui": { component: "tags", placeholder: "server-name", order: 1 },
      },
      detect_typosquats: {
        type: "boolean",
        title: "Detect Typosquats",
        description: "Detect server names similar to allowed servers",
        default: true,
        "x-ui": { order: 2 },
      },
      similarity_threshold: {
        type: "number",
        title: "Similarity Threshold",
        description: "Levenshtein similarity threshold for typosquat detection",
        default: 0.85,
        minimum: 0,
        maximum: 1,
        "x-ui": { component: "slider", order: 3 },
      },
    },
    "x-guard-meta": {
      guardType: "server_whitelist",
      version: "1.0.0",
      category: "prevention",
      defaultRunsOn: ["connection"],
      icon: "list-check",
    },
  },

  pii: {
    $id: "agentgateway://guards/pii/v1",
    title: "PII Detection",
    description: "Detects and masks personally identifiable information",
    type: "object",
    properties: {
      detect: {
        type: "array",
        title: "PII Types to Detect",
        description: "Types of PII to detect in responses",
        items: {
          type: "string",
          enum: ["email", "phone_number", "ssn", "credit_card", "ca_sin", "url"],
        },
        default: ["email", "phone_number", "ssn", "credit_card"],
        "x-ui": {
          component: "multiselect",
          labels: {
            email: "Email Address",
            phone_number: "Phone Number",
            ssn: "Social Security Number",
            credit_card: "Credit Card",
            ca_sin: "Canadian SIN",
            url: "URL",
          },
          order: 1,
        },
      },
      action: {
        type: "string",
        title: "Action",
        description: "What to do when PII is detected",
        enum: ["mask", "reject"],
        default: "mask",
        "x-ui": {
          component: "select",
          labels: {
            mask: "Mask PII",
            reject: "Reject Request",
          },
          order: 2,
        },
      },
      min_score: {
        type: "number",
        title: "Minimum Confidence Score",
        description: "Minimum confidence score to trigger detection (0-1)",
        default: 0.8,
        minimum: 0,
        maximum: 1,
        "x-ui": { component: "slider", order: 3 },
      },
      rejection_message: {
        type: "string",
        title: "Rejection Message",
        description: "Custom message when rejecting requests (optional)",
        "x-ui": { placeholder: "PII detected in response", order: 4, advanced: true },
      },
    },
    "x-guard-meta": {
      guardType: "pii",
      version: "1.0.0",
      category: "modification",
      defaultRunsOn: ["response"],
      icon: "user-x",
    },
  },
};

// =============================================================================
// Hook Implementation
// =============================================================================

interface UseGuardSchemasResult {
  /** Map of guard type to schema */
  schemas: Record<string, GuardSettingsSchema>;

  /** List of available guard types with metadata */
  availableGuards: GuardTypeSummary[];

  /** Loading state */
  loading: boolean;

  /** Error if fetch failed */
  error: Error | null;

  /** Get schema for a specific guard type */
  getSchema: (guardType: string) => GuardSettingsSchema | undefined;

  /** Resolve config with defaults from schema */
  resolveConfig: (
    guardType: string,
    config: Record<string, unknown>
  ) => Record<string, unknown>;

  /** Refresh schemas from API */
  refresh: () => Promise<void>;
}

/**
 * Hook for accessing guard schemas
 *
 * @param fetchFromApi - Whether to fetch schemas from API (default: false, uses embedded)
 */
export function useGuardSchemas(fetchFromApi = false): UseGuardSchemasResult {
  const [schemas, setSchemas] = useState<Record<string, GuardSettingsSchema>>(
    NATIVE_GUARD_SCHEMAS
  );
  const [loading, setLoading] = useState(fetchFromApi);
  const [error, setError] = useState<Error | null>(null);

  const fetchSchemas = useCallback(async () => {
    if (!fetchFromApi) {
      // Use embedded schemas
      setSchemas(NATIVE_GUARD_SCHEMAS);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_URL}/api/v1/guards/schemas`);
      if (!response.ok) {
        throw new Error(`Failed to fetch guard schemas: ${response.status}`);
      }

      const data: GuardSchemasResponse = await response.json();
      // Merge API schemas with native schemas (API takes precedence)
      setSchemas({ ...NATIVE_GUARD_SCHEMAS, ...data.schemas });
    } catch (err) {
      console.warn("Failed to fetch guard schemas from API, using embedded:", err);
      setError(err instanceof Error ? err : new Error(String(err)));
      // Fall back to embedded schemas
      setSchemas(NATIVE_GUARD_SCHEMAS);
    } finally {
      setLoading(false);
    }
  }, [fetchFromApi]);

  useEffect(() => {
    fetchSchemas();
  }, [fetchSchemas]);

  const getSchema = useCallback(
    (guardType: string): GuardSettingsSchema | undefined => {
      return schemas[guardType];
    },
    [schemas]
  );

  const resolveConfig = useCallback(
    (guardType: string, config: Record<string, unknown>): Record<string, unknown> => {
      const schema = schemas[guardType];
      if (!schema) {
        return config;
      }
      return resolveWithDefaults(config, schema);
    },
    [schemas]
  );

  const availableGuards: GuardTypeSummary[] = Object.entries(schemas).map(
    ([type, schema]) => ({
      type,
      title: schema.title,
      description: schema.description,
      category: schema["x-guard-meta"]?.category || "detection",
      icon: schema["x-guard-meta"]?.icon,
      isWasm: !NATIVE_GUARD_SCHEMAS[type],
    })
  );

  return {
    schemas,
    availableGuards,
    loading,
    error,
    getSchema,
    resolveConfig,
    refresh: fetchSchemas,
  };
}

/**
 * Get embedded schema for a guard type without using React hooks
 */
export function getEmbeddedSchema(guardType: string): GuardSettingsSchema | undefined {
  return NATIVE_GUARD_SCHEMAS[guardType];
}

/**
 * Get all embedded guard types
 */
export function getEmbeddedGuardTypes(): string[] {
  return Object.keys(NATIVE_GUARD_SCHEMAS);
}
