/**
 * TextareaField - Multi-line text input for long string schema types
 */

import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { type SchemaProperty } from "@/lib/guard-schema-types";

interface TextareaFieldProps {
  name: string;
  schema: SchemaProperty;
  value: string;
  onChange: (value: string) => void;
  error?: string;
  disabled?: boolean;
}

export function TextareaField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: TextareaFieldProps) {
  const placeholder = schema["x-ui"]?.placeholder;
  const rows = schema["x-ui"]?.rows ?? 4;

  return (
    <div className="space-y-2">
      <Label htmlFor={name}>{schema.title || name}</Label>
      {schema.description && <p className="text-xs text-muted-foreground">{schema.description}</p>}
      <Textarea
        id={name}
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        rows={rows}
        className={error ? "border-destructive" : ""}
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
