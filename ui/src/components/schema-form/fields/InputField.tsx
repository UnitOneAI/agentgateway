/**
 * InputField - Text/number input for string and number schema types
 */

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { type SchemaProperty } from "@/lib/guard-schema-types";

interface InputFieldProps {
  name: string;
  schema: SchemaProperty;
  value: unknown;
  onChange: (value: unknown) => void;
  error?: string;
  disabled?: boolean;
}

export function InputField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: InputFieldProps) {
  const isNumber = schema.type === "number" || schema.type === "integer";
  const placeholder = schema["x-ui"]?.placeholder;

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const rawValue = e.target.value;
    if (isNumber) {
      if (rawValue === "") {
        onChange(undefined);
      } else {
        const num = schema.type === "integer" ? parseInt(rawValue, 10) : parseFloat(rawValue);
        if (!isNaN(num)) {
          onChange(num);
        }
      }
    } else {
      onChange(rawValue);
    }
  };

  return (
    <div className="space-y-2">
      <Label htmlFor={name}>
        {schema.title || name}
      </Label>
      {schema.description && (
        <p className="text-xs text-muted-foreground">{schema.description}</p>
      )}
      <Input
        id={name}
        type={isNumber ? "number" : "text"}
        value={value !== undefined && value !== null ? String(value) : ""}
        onChange={handleChange}
        placeholder={placeholder}
        disabled={disabled}
        min={isNumber && "minimum" in schema ? schema.minimum : undefined}
        max={isNumber && "maximum" in schema ? schema.maximum : undefined}
        step={schema.type === "integer" ? 1 : "any"}
        className={error ? "border-destructive" : ""}
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
