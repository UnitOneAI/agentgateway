/**
 * CheckboxField - Boolean toggle for boolean schema types
 */

import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { type SchemaProperty } from "@/lib/guard-schema-types";

interface CheckboxFieldProps {
  name: string;
  schema: SchemaProperty;
  value: boolean;
  onChange: (value: boolean) => void;
  error?: string;
  disabled?: boolean;
}

export function CheckboxField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: CheckboxFieldProps) {
  return (
    <div className="space-y-2">
      <div className="flex items-center space-x-2">
        <Checkbox
          id={name}
          checked={value ?? false}
          onCheckedChange={(checked) => onChange(checked === true)}
          disabled={disabled}
        />
        <Label htmlFor={name} className="cursor-pointer">
          {schema.title || name}
        </Label>
      </div>
      {schema.description && (
        <p className="text-xs text-muted-foreground ml-6">{schema.description}</p>
      )}
      {error && <p className="text-xs text-destructive ml-6">{error}</p>}
    </div>
  );
}
