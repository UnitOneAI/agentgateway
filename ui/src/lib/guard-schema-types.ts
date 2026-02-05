/**
 * TypeScript types for Guard Settings Schema (JSON Schema based)
 *
 * These types define the schema structure that guards export to describe
 * their configurable parameters. The UI uses these schemas to generate
 * dynamic configuration forms.
 */

// =============================================================================
// UI Hints Extension
// =============================================================================

/**
 * UI rendering hints for schema properties.
 * These extend JSON Schema with UI-specific metadata.
 */
export interface SchemaUIHints {
  /** Preferred UI component to render this field */
  component?:
    | "input"
    | "textarea"
    | "select"
    | "checkbox"
    | "slider"
    | "tags"
    | "multiselect"
    | "object-array"
    | "key-value";

  /** Placeholder text for input fields */
  placeholder?: string;

  /** Help text shown below the field */
  helpText?: string;

  /** Show in advanced section (collapsed by default) */
  advanced?: boolean;

  /** Display order within the group (lower = first) */
  order?: number;

  /** Group name for organizing related fields */
  group?: string;

  /** Number of rows for textarea */
  rows?: number;

  /** Display labels for enum values */
  labels?: Record<string, string>;
}

/**
 * UI group definition for organizing schema properties
 */
export interface SchemaUIGroup {
  title: string;
  order: number;
  description?: string;
  collapsed?: boolean;
}

// =============================================================================
// Guard Metadata Extension
// =============================================================================

/**
 * Guard metadata embedded in the schema.
 * Provides information about the guard for UI display and filtering.
 */
export interface GuardMeta {
  /** Guard type identifier (e.g., "server_spoofing", "tool_poisoning") */
  guardType: string;

  /** Schema version (semver) */
  version: string;

  /** Guard category for UI grouping */
  category: "detection" | "prevention" | "modification" | "logging";

  /** Default guard phases to run on */
  defaultRunsOn: GuardPhase[];

  /** Icon identifier for UI display */
  icon?: string;
}

export type GuardPhase = "connection" | "request" | "response" | "tools_list" | "tool_invoke";

// =============================================================================
// JSON Schema Types (Draft 2020-12 subset)
// =============================================================================

/**
 * Base schema property interface
 */
export interface SchemaPropertyBase {
  /** Property type */
  type: "string" | "number" | "integer" | "boolean" | "array" | "object";

  /** Human-readable title */
  title?: string;

  /** Description of the property */
  description?: string;

  /** Default value */
  default?: unknown;

  /** UI rendering hints */
  "x-ui"?: SchemaUIHints;
}

/**
 * String property schema
 */
export interface StringSchemaProperty extends SchemaPropertyBase {
  type: "string";
  default?: string;

  /** Allowed values (renders as select/radio) */
  enum?: string[];

  /** Minimum length */
  minLength?: number;

  /** Maximum length */
  maxLength?: number;

  /** Regex pattern for validation */
  pattern?: string;

  /** String format hint */
  format?: "uri" | "email" | "date" | "time" | "date-time" | "regex";
}

/**
 * Number/Integer property schema
 */
export interface NumberSchemaProperty extends SchemaPropertyBase {
  type: "number" | "integer";
  default?: number;

  /** Allowed values */
  enum?: number[];

  /** Minimum value (inclusive) */
  minimum?: number;

  /** Maximum value (inclusive) */
  maximum?: number;

  /** Exclusive minimum */
  exclusiveMinimum?: number;

  /** Exclusive maximum */
  exclusiveMaximum?: number;

  /** Value must be multiple of this */
  multipleOf?: number;
}

/**
 * Boolean property schema
 */
export interface BooleanSchemaProperty extends SchemaPropertyBase {
  type: "boolean";
  default?: boolean;
}

/**
 * Array property schema
 */
export interface ArraySchemaProperty extends SchemaPropertyBase {
  type: "array";
  default?: unknown[];

  /** Schema for array items */
  items?: SchemaProperty;

  /** Minimum number of items */
  minItems?: number;

  /** Maximum number of items */
  maxItems?: number;

  /** Items must be unique */
  uniqueItems?: boolean;
}

/**
 * Object property schema
 */
export interface ObjectSchemaProperty extends SchemaPropertyBase {
  type: "object";
  default?: Record<string, unknown>;

  /** Nested property definitions */
  properties?: Record<string, SchemaProperty>;

  /** Required property names */
  required?: string[];

  /** Schema for additional properties (for key-value maps) */
  additionalProperties?: SchemaProperty | boolean;
}

/**
 * Union type for all schema property types
 */
export type SchemaProperty =
  | StringSchemaProperty
  | NumberSchemaProperty
  | BooleanSchemaProperty
  | ArraySchemaProperty
  | ObjectSchemaProperty;

// =============================================================================
// Guard Settings Schema
// =============================================================================

/**
 * Complete guard settings schema.
 *
 * This is the root schema object that guards export to describe their
 * configurable parameters. It follows JSON Schema Draft 2020-12 with
 * custom extensions for UI rendering.
 */
export interface GuardSettingsSchema {
  /** JSON Schema version */
  $schema?: string;

  /** Schema identifier */
  $id?: string;

  /** Guard display name */
  title: string;

  /** Guard description */
  description?: string;

  /** Root type (always "object" for guard schemas) */
  type: "object";

  /** Guard-specific property definitions */
  properties: Record<string, SchemaProperty>;

  /** Required property names */
  required?: string[];

  /** UI group definitions */
  "x-ui-groups"?: Record<string, SchemaUIGroup>;

  /** Guard metadata */
  "x-guard-meta"?: GuardMeta;
}

// =============================================================================
// Schema Registry Types
// =============================================================================

/**
 * Summary of an available guard type
 */
export interface GuardTypeSummary {
  /** Guard type identifier */
  type: string;

  /** Display name */
  title: string;

  /** Description */
  description?: string;

  /** Category */
  category: string;

  /** Icon identifier */
  icon?: string;

  /** Whether this is a WASM guard */
  isWasm: boolean;
}

/**
 * Response from the guard schemas API endpoint
 */
export interface GuardSchemasResponse {
  /** Map of guard type to schema */
  schemas: Record<string, GuardSettingsSchema>;

  /** List of available guard types */
  availableGuards: GuardTypeSummary[];
}

// =============================================================================
// Utility Types
// =============================================================================

/**
 * Resolved guard configuration with defaults applied
 */
export type ResolvedGuardConfig<T = Record<string, unknown>> = T;

/**
 * Validation error for a specific field
 */
export interface SchemaValidationError {
  /** Property path (dot-separated for nested) */
  path: string;

  /** Error message */
  message: string;

  /** Error code */
  code: "required" | "type" | "minimum" | "maximum" | "pattern" | "enum" | "custom";
}

/**
 * Result of schema validation
 */
export interface SchemaValidationResult {
  valid: boolean;
  errors: SchemaValidationError[];
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Infer the appropriate UI component from a schema property
 */
export function inferUIComponent(schema: SchemaProperty): string {
  const hint = schema["x-ui"]?.component;
  if (hint) return hint;

  switch (schema.type) {
    case "boolean":
      return "checkbox";
    case "string":
      if ("enum" in schema && schema.enum) return "select";
      if ("maxLength" in schema && (schema.maxLength ?? 0) > 200) return "textarea";
      return "input";
    case "number":
    case "integer":
      if ("enum" in schema && schema.enum) return "select";
      if ("minimum" in schema && "maximum" in schema) return "slider";
      return "input";
    case "array":
      if ("items" in schema && schema.items) {
        if (schema.items.type === "string" && "enum" in schema.items) {
          return "multiselect";
        }
        if (schema.items.type === "object") return "object-array";
      }
      return "tags";
    case "object":
      if ("additionalProperties" in schema && schema.additionalProperties) {
        return "key-value";
      }
      return "object";
    default:
      return "input";
  }
}

/**
 * Resolve configuration with defaults from schema
 */
export function resolveWithDefaults(
  config: Record<string, unknown>,
  schema: GuardSettingsSchema
): Record<string, unknown> {
  const result: Record<string, unknown> = { ...config };

  for (const [key, prop] of Object.entries(schema.properties)) {
    if (!(key in result) && prop.default !== undefined) {
      result[key] = structuredClone(prop.default);
    }
  }

  return result;
}

/**
 * Get properties sorted by UI order
 */
export function getSortedProperties(schema: GuardSettingsSchema): Array<[string, SchemaProperty]> {
  return Object.entries(schema.properties).sort(([, a], [, b]) => {
    const orderA = a["x-ui"]?.order ?? 999;
    const orderB = b["x-ui"]?.order ?? 999;
    return orderA - orderB;
  });
}

/**
 * Group properties by their x-ui.group value
 */
export function getGroupedProperties(
  schema: GuardSettingsSchema
): Map<string | undefined, Array<[string, SchemaProperty]>> {
  const groups = new Map<string | undefined, Array<[string, SchemaProperty]>>();
  const sorted = getSortedProperties(schema);

  for (const entry of sorted) {
    const group = entry[1]["x-ui"]?.group;
    if (!groups.has(group)) {
      groups.set(group, []);
    }
    groups.get(group)!.push(entry);
  }

  return groups;
}
