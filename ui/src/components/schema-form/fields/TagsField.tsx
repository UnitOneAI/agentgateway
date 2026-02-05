/**
 * TagsField - Tag input for string array schema types
 */

import { useState, KeyboardEvent } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { X } from "lucide-react";
import { type SchemaProperty } from "@/lib/guard-schema-types";

interface TagsFieldProps {
  name: string;
  schema: SchemaProperty;
  value: string[];
  onChange: (value: string[]) => void;
  error?: string;
  disabled?: boolean;
}

export function TagsField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled,
}: TagsFieldProps) {
  const [inputValue, setInputValue] = useState("");
  const items = Array.isArray(value) ? value : [];
  const placeholder = schema["x-ui"]?.placeholder || "Type and press Enter";

  const addTag = (tag: string) => {
    const trimmed = tag.trim();
    if (trimmed && !items.includes(trimmed)) {
      onChange([...items, trimmed]);
    }
    setInputValue("");
  };

  const removeTag = (index: number) => {
    const newItems = items.filter((_, i) => i !== index);
    onChange(newItems);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      addTag(inputValue);
    } else if (e.key === "Backspace" && inputValue === "" && items.length > 0) {
      removeTag(items.length - 1);
    }
  };

  const handleBlur = () => {
    if (inputValue.trim()) {
      addTag(inputValue);
    }
  };

  return (
    <div className="space-y-2">
      <Label htmlFor={name}>{schema.title || name}</Label>
      {schema.description && (
        <p className="text-xs text-muted-foreground">{schema.description}</p>
      )}
      <div className="space-y-2">
        {items.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {items.map((item, index) => (
              <Badge key={index} variant="secondary" className="gap-1">
                {item}
                {!disabled && (
                  <button
                    type="button"
                    onClick={() => removeTag(index)}
                    className="ml-1 hover:text-destructive"
                  >
                    <X className="h-3 w-3" />
                  </button>
                )}
              </Badge>
            ))}
          </div>
        )}
        <Input
          id={name}
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          placeholder={placeholder}
          disabled={disabled}
          className={error ? "border-destructive" : ""}
        />
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
