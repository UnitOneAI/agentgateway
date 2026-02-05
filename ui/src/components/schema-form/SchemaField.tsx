/**
 * SchemaField - Routes schema properties to appropriate field components
 */

import { type SchemaProperty, inferUIComponent } from "@/lib/guard-schema-types";
import { InputField } from "./fields/InputField";
import { CheckboxField } from "./fields/CheckboxField";
import { SelectField } from "./fields/SelectField";
import { SliderField } from "./fields/SliderField";
import { TagsField } from "./fields/TagsField";
import { MultiselectField } from "./fields/MultiselectField";
import { ObjectArrayField } from "./fields/ObjectArrayField";
import { KeyValueField } from "./fields/KeyValueField";
import { TextareaField } from "./fields/TextareaField";

export interface SchemaFieldProps {
  /** Field name (property key) */
  name: string;

  /** Schema definition for this field */
  schema: SchemaProperty;

  /** Current value */
  value: unknown;

  /** Change handler */
  onChange: (value: unknown) => void;

  /** Error message */
  error?: string;

  /** Disable the field */
  disabled?: boolean;
}

/**
 * Routes a schema property to the appropriate field component
 */
export function SchemaField({
  name,
  schema,
  value,
  onChange,
  error,
  disabled = false,
}: SchemaFieldProps) {
  const component = inferUIComponent(schema);

  const commonProps = {
    name,
    schema,
    value,
    onChange,
    error,
    disabled,
  };

  switch (component) {
    case "checkbox":
      return <CheckboxField {...commonProps} value={value as boolean} />;

    case "select":
      return <SelectField {...commonProps} value={value as string | number} />;

    case "slider":
      return <SliderField {...commonProps} value={value as number} />;

    case "tags":
      return <TagsField {...commonProps} value={value as string[]} />;

    case "multiselect":
      return <MultiselectField {...commonProps} value={value as string[]} />;

    case "object-array":
      return <ObjectArrayField {...commonProps} value={value as Record<string, unknown>[]} />;

    case "key-value":
      return <KeyValueField {...commonProps} value={value as Record<string, string>} />;

    case "textarea":
      return <TextareaField {...commonProps} value={value as string} />;

    case "input":
    default:
      return <InputField {...commonProps} />;
  }
}
