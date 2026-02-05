/**
 * KeyValueField - Key-value pair editor for object maps
 */

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Plus, Trash2 } from "lucide-react";
import { type SchemaProperty } from "@/lib/guard-schema-types";

interface KeyValueFieldProps {
  name: string;
  schema: SchemaProperty;
  value: Record<string, string>;
  onChange: (value: Record<string, string>) => void;
  error?: string;
  disabled?: boolean;
}

export function KeyValueField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: KeyValueFieldProps) {
  const entries = Object.entries(value || {});

  const addEntry = () => {
    const newKey = `key${entries.length + 1}`;
    onChange({ ...value, [newKey]: "" });
  };

  const removeEntry = (key: string) => {
    const newValue = { ...value };
    delete newValue[key];
    onChange(newValue);
  };

  const updateKey = (oldKey: string, newKey: string) => {
    if (oldKey === newKey) return;
    const newValue: Record<string, string> = {};
    for (const [k, v] of Object.entries(value || {})) {
      if (k === oldKey) {
        newValue[newKey] = v;
      } else {
        newValue[k] = v;
      }
    }
    onChange(newValue);
  };

  const updateValue = (key: string, newVal: string) => {
    onChange({ ...value, [key]: newVal });
  };

  return (
    <div className="space-y-2">
      <Label>{schema.title || name}</Label>
      {schema.description && <p className="text-xs text-muted-foreground">{schema.description}</p>}
      <div className="space-y-2">
        {entries.map(([key, val], index) => (
          <div key={index} className="flex items-center space-x-2">
            <Input
              value={key}
              onChange={(e) => updateKey(key, e.target.value)}
              placeholder="Key"
              disabled={disabled}
              className="flex-1"
            />
            <Input
              value={val}
              onChange={(e) => updateValue(key, e.target.value)}
              placeholder="Value"
              disabled={disabled}
              className="flex-1"
            />
            {!disabled && (
              <Button variant="ghost" size="sm" onClick={() => removeEntry(key)}>
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        ))}
        {!disabled && (
          <Button variant="outline" size="sm" onClick={addEntry}>
            <Plus className="h-4 w-4 mr-2" />
            Add Entry
          </Button>
        )}
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
