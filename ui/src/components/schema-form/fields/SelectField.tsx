/**
 * SelectField - Dropdown select for enum schema types
 */

import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type SchemaProperty,
  type StringSchemaProperty,
  type NumberSchemaProperty,
} from "@/lib/guard-schema-types";

interface SelectFieldProps {
  name: string;
  schema: SchemaProperty;
  value: string | number;
  onChange: (value: string | number) => void;
  error?: string;
  disabled?: boolean;
}

export function SelectField({ name, schema, value, onChange, error, disabled }: SelectFieldProps) {
  // Get enum values from schema
  const enumValues: (string | number)[] =
    schema.type === "string"
      ? (schema as StringSchemaProperty).enum || []
      : (schema as NumberSchemaProperty).enum || [];

  // Get display labels from UI hints
  const labels = schema["x-ui"]?.labels || {};

  const handleChange = (newValue: string) => {
    if (schema.type === "number" || schema.type === "integer") {
      onChange(parseFloat(newValue));
    } else {
      onChange(newValue);
    }
  };

  return (
    <div className="space-y-2">
      <Label htmlFor={name}>{schema.title || name}</Label>
      {schema.description && <p className="text-xs text-muted-foreground">{schema.description}</p>}
      <Select
        value={value !== undefined ? String(value) : undefined}
        onValueChange={handleChange}
        disabled={disabled}
      >
        <SelectTrigger id={name} className={error ? "border-destructive" : ""}>
          <SelectValue placeholder={schema["x-ui"]?.placeholder || "Select..."} />
        </SelectTrigger>
        <SelectContent>
          {enumValues.map((enumValue) => (
            <SelectItem key={String(enumValue)} value={String(enumValue)}>
              {labels[String(enumValue)] || String(enumValue)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
