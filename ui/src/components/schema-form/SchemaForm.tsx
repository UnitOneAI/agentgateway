/**
 * SchemaForm - Dynamic form generator from JSON Schema
 *
 * This component renders a form based on a JSON Schema definition,
 * automatically selecting appropriate field components based on type
 * and UI hints.
 */

import { useMemo, useState } from "react";
import {
  type GuardSettingsSchema,
  type SchemaProperty,
  getSortedProperties,
  getGroupedProperties,
} from "@/lib/guard-schema-types";
import { SchemaField } from "./SchemaField";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronRight } from "lucide-react";

interface SchemaFormProps {
  /** JSON Schema defining the form fields */
  schema: GuardSettingsSchema;

  /** Current form values */
  value: Record<string, unknown>;

  /** Callback when any value changes */
  onChange: (value: Record<string, unknown>) => void;

  /** Validation errors by field path */
  errors?: Record<string, string>;

  /** Show advanced fields by default */
  showAdvanced?: boolean;

  /** Disable all fields */
  disabled?: boolean;
}

/**
 * Dynamic form component that renders fields from a JSON Schema
 */
export function SchemaForm({
  schema,
  value,
  onChange,
  errors = {},
  showAdvanced = false,
  disabled = false,
}: SchemaFormProps) {
  const [advancedExpanded, setAdvancedExpanded] = useState(showAdvanced);

  // Group properties by their x-ui.group value
  const groupedProperties = useMemo(() => getGroupedProperties(schema), [schema]);

  // Get group definitions from schema
  const groupDefs = schema["x-ui-groups"] || {};

  // Sort groups by their order
  const sortedGroups = useMemo(() => {
    const groups = Array.from(groupedProperties.keys());
    return groups.sort((a, b) => {
      const orderA = a ? (groupDefs[a]?.order ?? 999) : 0;
      const orderB = b ? (groupDefs[b]?.order ?? 999) : 0;
      return orderA - orderB;
    });
  }, [groupedProperties, groupDefs]);

  // Separate advanced fields
  const { regularFields, advancedFields } = useMemo(() => {
    const regular: Array<[string, SchemaProperty]> = [];
    const advanced: Array<[string, SchemaProperty]> = [];

    for (const [key, prop] of getSortedProperties(schema)) {
      if (prop["x-ui"]?.advanced) {
        advanced.push([key, prop]);
      } else {
        regular.push([key, prop]);
      }
    }

    return { regularFields: regular, advancedFields: advanced };
  }, [schema]);

  const handleFieldChange = (key: string, newValue: unknown) => {
    onChange({ ...value, [key]: newValue });
  };

  const renderField = (key: string, prop: SchemaProperty) => (
    <SchemaField
      key={key}
      name={key}
      schema={prop}
      value={value[key] ?? prop.default}
      onChange={(newValue) => handleFieldChange(key, newValue)}
      error={errors[key]}
      disabled={disabled}
    />
  );

  // Check if we're using groups
  const hasGroups = sortedGroups.some((g) => g !== undefined);

  if (hasGroups) {
    return (
      <div className="space-y-6">
        {sortedGroups.map((groupName) => {
          const groupProps = groupedProperties.get(groupName) || [];
          const groupDef = groupName ? groupDefs[groupName] : null;

          // Separate regular and advanced within group
          const regular = groupProps.filter(([, p]) => !p["x-ui"]?.advanced);
          const advanced = groupProps.filter(([, p]) => p["x-ui"]?.advanced);

          if (regular.length === 0 && advanced.length === 0) return null;

          return (
            <div key={groupName ?? "default"} className="space-y-4">
              {groupDef && (
                <div className="border-b pb-2">
                  <h4 className="text-sm font-medium">{groupDef.title}</h4>
                  {groupDef.description && (
                    <p className="text-xs text-muted-foreground mt-1">{groupDef.description}</p>
                  )}
                </div>
              )}
              <div className="space-y-4 pl-1">
                {regular.map(([key, prop]) => renderField(key, prop))}
              </div>
              {advanced.length > 0 && (
                <Collapsible open={advancedExpanded} onOpenChange={setAdvancedExpanded}>
                  <CollapsibleTrigger asChild>
                    <Button variant="ghost" size="sm" className="text-xs text-muted-foreground">
                      {advancedExpanded ? (
                        <ChevronDown className="h-3 w-3 mr-1" />
                      ) : (
                        <ChevronRight className="h-3 w-3 mr-1" />
                      )}
                      Advanced options ({advanced.length})
                    </Button>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="space-y-4 pl-1 pt-2">
                    {advanced.map(([key, prop]) => renderField(key, prop))}
                  </CollapsibleContent>
                </Collapsible>
              )}
            </div>
          );
        })}
      </div>
    );
  }

  // No groups - render flat list
  return (
    <div className="space-y-4">
      {regularFields.map(([key, prop]) => renderField(key, prop))}

      {advancedFields.length > 0 && (
        <Collapsible open={advancedExpanded} onOpenChange={setAdvancedExpanded}>
          <CollapsibleTrigger asChild>
            <Button variant="ghost" size="sm" className="text-xs text-muted-foreground">
              {advancedExpanded ? (
                <ChevronDown className="h-3 w-3 mr-1" />
              ) : (
                <ChevronRight className="h-3 w-3 mr-1" />
              )}
              Advanced options ({advancedFields.length})
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="space-y-4 pt-2">
            {advancedFields.map(([key, prop]) => renderField(key, prop))}
          </CollapsibleContent>
        </Collapsible>
      )}
    </div>
  );
}
