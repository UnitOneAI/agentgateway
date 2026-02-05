/**
 * MultiselectField - Checkbox group for enum array schema types
 */

import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  type SchemaProperty,
  type ArraySchemaProperty,
  type StringSchemaProperty,
} from "@/lib/guard-schema-types";

interface MultiselectFieldProps {
  name: string;
  schema: SchemaProperty;
  value: string[];
  onChange: (value: string[]) => void;
  error?: string;
  disabled?: boolean;
}

export function MultiselectField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: MultiselectFieldProps) {
  const items = Array.isArray(value) ? value : [];

  // Get enum values from items schema
  const arraySchema = schema as ArraySchemaProperty;
  const itemsSchema = arraySchema.items as StringSchemaProperty | undefined;
  const enumValues = itemsSchema?.enum || [];

  // Get display labels from UI hints
  const labels = schema["x-ui"]?.labels || {};

  const handleToggle = (enumValue: string, checked: boolean) => {
    if (checked) {
      onChange([...items, enumValue]);
    } else {
      onChange(items.filter((item) => item !== enumValue));
    }
  };

  return (
    <div className="space-y-2">
      <Label>{schema.title || name}</Label>
      {schema.description && <p className="text-xs text-muted-foreground">{schema.description}</p>}
      <div className="space-y-2">
        {enumValues.map((enumValue) => (
          <div key={enumValue} className="flex items-center space-x-2">
            <Checkbox
              id={`${name}-${enumValue}`}
              checked={items.includes(enumValue)}
              onCheckedChange={(checked) => handleToggle(enumValue, checked === true)}
              disabled={disabled}
            />
            <Label htmlFor={`${name}-${enumValue}`} className="cursor-pointer font-normal">
              {labels[enumValue] || enumValue}
            </Label>
          </div>
        ))}
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
