/**
 * SliderField - Range slider for bounded number schema types
 */

import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { type SchemaProperty, type NumberSchemaProperty } from "@/lib/guard-schema-types";

interface SliderFieldProps {
  name: string;
  schema: SchemaProperty;
  value: number;
  onChange: (value: number) => void;
  error?: string;
  disabled?: boolean;
}

export function SliderField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: SliderFieldProps) {
  const numSchema = schema as NumberSchemaProperty;
  const min = numSchema.minimum ?? 0;
  const max = numSchema.maximum ?? 100;
  const step = numSchema.multipleOf ?? (schema.type === "integer" ? 1 : 0.01);

  // Local state for the input field to allow typing
  const [inputValue, setInputValue] = useState(String(value ?? min));

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const num = parseFloat(e.target.value);
    onChange(num);
    setInputValue(String(num));
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setInputValue(e.target.value);
  };

  const handleInputBlur = () => {
    let num = parseFloat(inputValue);
    if (isNaN(num)) {
      num = value ?? min;
    }
    // Clamp to range
    num = Math.max(min, Math.min(max, num));
    onChange(num);
    setInputValue(String(num));
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label htmlFor={name}>{schema.title || name}</Label>
        <Input
          type="number"
          value={inputValue}
          onChange={handleInputChange}
          onBlur={handleInputBlur}
          disabled={disabled}
          min={min}
          max={max}
          step={step}
          className="w-20 h-7 text-sm text-right"
        />
      </div>
      {schema.description && (
        <p className="text-xs text-muted-foreground">{schema.description}</p>
      )}
      <div className="flex items-center space-x-2">
        <span className="text-xs text-muted-foreground w-8">{min}</span>
        <input
          type="range"
          id={name}
          value={value ?? min}
          onChange={handleSliderChange}
          disabled={disabled}
          min={min}
          max={max}
          step={step}
          className="flex-1 h-2 bg-secondary rounded-lg appearance-none cursor-pointer accent-primary disabled:opacity-50 disabled:cursor-not-allowed"
        />
        <span className="text-xs text-muted-foreground w-8 text-right">{max}</span>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
