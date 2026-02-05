/**
 * ObjectArrayField - Expandable list for array of objects schema types
 */

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ChevronDown, ChevronRight, Plus, Trash2 } from "lucide-react";
import {
  type SchemaProperty,
  type ArraySchemaProperty,
  type ObjectSchemaProperty,
} from "@/lib/guard-schema-types";

interface ObjectArrayFieldProps {
  name: string;
  schema: SchemaProperty;
  value: Record<string, unknown>[];
  onChange: (value: Record<string, unknown>[]) => void;
  error?: string;
  disabled?: boolean;
}

export function ObjectArrayField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: ObjectArrayFieldProps) {
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set([0]));
  const items = Array.isArray(value) ? value : [];

  // Get item schema
  const arraySchema = schema as ArraySchemaProperty;
  const itemSchema = arraySchema.items as ObjectSchemaProperty | undefined;
  const itemProperties = itemSchema?.properties || {};
  const requiredFields = itemSchema?.required || [];

  const toggleExpanded = (index: number) => {
    const newExpanded = new Set(expandedItems);
    if (newExpanded.has(index)) {
      newExpanded.delete(index);
    } else {
      newExpanded.add(index);
    }
    setExpandedItems(newExpanded);
  };

  const addItem = () => {
    // Create new item with defaults
    const newItem: Record<string, unknown> = {};
    for (const [key, prop] of Object.entries(itemProperties)) {
      if (prop.default !== undefined) {
        newItem[key] = prop.default;
      }
    }
    const newItems = [...items, newItem];
    onChange(newItems);
    setExpandedItems(new Set([...expandedItems, newItems.length - 1]));
  };

  const removeItem = (index: number) => {
    const newItems = items.filter((_, i) => i !== index);
    onChange(newItems);
    const newExpanded = new Set(expandedItems);
    newExpanded.delete(index);
    setExpandedItems(newExpanded);
  };

  const updateItem = (index: number, key: string, newValue: unknown) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [key]: newValue };
    onChange(newItems);
  };

  const getItemLabel = (item: Record<string, unknown>, index: number): string => {
    // Try to find a name/title field for display
    const labelField = Object.keys(itemProperties).find(
      (k) => k === "name" || k === "title" || k === "id"
    );
    if (labelField && item[labelField]) {
      return String(item[labelField]);
    }
    return `Item ${index + 1}`;
  };

  return (
    <div className="space-y-2">
      <Label>{schema.title || name}</Label>
      {schema.description && (
        <p className="text-xs text-muted-foreground">{schema.description}</p>
      )}
      <div className="space-y-2">
        {items.map((item, index) => (
          <Card key={index} className="overflow-hidden">
            <Collapsible
              open={expandedItems.has(index)}
              onOpenChange={() => toggleExpanded(index)}
            >
              <CollapsibleTrigger asChild>
                <div className="flex items-center justify-between p-3 cursor-pointer hover:bg-accent/50">
                  <div className="flex items-center gap-2">
                    {expandedItems.has(index) ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                    <span className="text-sm font-medium">{getItemLabel(item, index)}</span>
                  </div>
                  {!disabled && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        removeItem(index);
                      }}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent>
                <CardContent className="pt-0 pb-3 space-y-3">
                  {Object.entries(itemProperties).map(([key, prop]) => (
                    <div key={key} className="space-y-1">
                      <Label htmlFor={`${name}-${index}-${key}`} className="text-xs">
                        {prop.title || key}
                        {requiredFields.includes(key) && " *"}
                      </Label>
                      {prop.type === "boolean" ? (
                        <div className="flex items-center space-x-2">
                          <input
                            type="checkbox"
                            id={`${name}-${index}-${key}`}
                            checked={item[key] as boolean ?? false}
                            onChange={(e) => updateItem(index, key, e.target.checked)}
                            disabled={disabled}
                            className="h-4 w-4"
                          />
                          {prop.description && (
                            <span className="text-xs text-muted-foreground">
                              {prop.description}
                            </span>
                          )}
                        </div>
                      ) : (
                        <Input
                          id={`${name}-${index}-${key}`}
                          type={prop.type === "number" || prop.type === "integer" ? "number" : "text"}
                          value={item[key] !== undefined ? String(item[key]) : ""}
                          onChange={(e) => {
                            const rawValue = e.target.value;
                            if (prop.type === "number" || prop.type === "integer") {
                              const num = parseFloat(rawValue);
                              updateItem(index, key, isNaN(num) ? undefined : num);
                            } else {
                              updateItem(index, key, rawValue);
                            }
                          }}
                          placeholder={prop["x-ui"]?.placeholder || prop.description}
                          disabled={disabled}
                        />
                      )}
                    </div>
                  ))}
                </CardContent>
              </CollapsibleContent>
            </Collapsible>
          </Card>
        ))}
        {!disabled && (
          <Button variant="outline" size="sm" onClick={addItem} className="w-full">
            <Plus className="h-4 w-4 mr-2" />
            Add {itemSchema?.title || "Item"}
          </Button>
        )}
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
