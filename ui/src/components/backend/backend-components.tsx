"use client";

import React from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Plus,
  Target,
  ChevronDown,
  ChevronRight,
  Trash2,
  Edit,
  Brain,
  Cloud,
  Server,
  Globe,
  Loader2,
  Shield,
  AlertTriangle,
  Eye,
  Copy,
  CheckCircle,
  ShieldAlert,
  Code,
} from "lucide-react";
import { Bind } from "@/lib/types";
import { BackendWithContext } from "@/lib/backend-hooks";
import {
  DEFAULT_BACKEND_FORM,
  BACKEND_TYPES,
  BACKEND_TABLE_HEADERS,
  HOST_TYPES,
  AI_MODEL_PLACEHOLDERS,
  AI_REGION_PLACEHOLDERS,
  SECURITY_GUARD_TYPES,
  GUARD_PHASES,
  FAILURE_MODES,
  PII_TYPES,
  PII_ACTIONS,
  SCAN_FIELDS,
} from "@/lib/backend-constants";
import { SchemaForm } from "@/components/schema-form";
import { useGuardSchemas } from "@/hooks/useGuardSchemas";
import type { SecurityGuard, SecurityGuardType } from "@/lib/types";
import {
  getBackendType,
  getBackendName,
  getBackendTypeColor,
  getBackendDetails,
  getAvailableRoutes,
  AI_PROVIDERS,
  MCP_TARGET_TYPES,
  hasBackendPolicies,
  getBackendPolicyTypes,
  canDeleteBackend,
} from "@/lib/backend-utils";
import { useXdsMode } from "@/hooks/use-xds-mode";

const getEnvAsRecord = (env: unknown): Record<string, string> => {
  return typeof env === "object" && env !== null ? (env as Record<string, string>) : {};
};

// Icon mapping
const getBackendIcon = (type: string) => {
  switch (type) {
    case "mcp":
      return <Target className="h-4 w-4" />;
    case "ai":
      return <Brain className="h-4 w-4" />;
    case "service":
      return <Cloud className="h-4 w-4" />;
    case "host":
      return <Server className="h-4 w-4" />;
    case "dynamic":
      return <Globe className="h-4 w-4" />;
    default:
      return <Server className="h-4 w-4" />;
  }
};

interface BackendTableProps {
  backendsByBind: Map<number, BackendWithContext[]>;
  expandedBinds: Set<number>;
  setExpandedBinds: React.Dispatch<React.SetStateAction<Set<number>>>;
  onEditBackend: (backendContext: BackendWithContext) => void;
  onDeleteBackend: (backendContext: BackendWithContext) => void;
  isSubmitting: boolean;
}

export const BackendTable: React.FC<BackendTableProps> = ({
  backendsByBind,
  expandedBinds,
  setExpandedBinds,
  onEditBackend,
  onDeleteBackend,
  isSubmitting,
}) => {
  const xds = useXdsMode();
  return (
    <div className="space-y-4">
      {Array.from(backendsByBind.entries()).map(([port, backendContexts]) => {
        const typeCounts = backendContexts.reduce(
          (acc, bc) => {
            const type = getBackendType(bc.backend);
            acc[type] = (acc[type] || 0) + 1;
            return acc;
          },
          {} as Record<string, number>
        );

        return (
          <Card key={port}>
            <Collapsible
              open={expandedBinds.has(port)}
              onOpenChange={() => {
                setExpandedBinds((prev) => {
                  const newSet = new Set(prev);
                  if (newSet.has(port)) {
                    newSet.delete(port);
                  } else {
                    newSet.add(port);
                  }
                  return newSet;
                });
              }}
            >
              <CollapsibleTrigger asChild>
                <CardHeader className="hover:bg-muted/50 cursor-pointer">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-4">
                      {expandedBinds.has(port) ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                      <div>
                        <h3 className="text-lg font-semibold">Port {port}</h3>
                        <div className="flex items-center space-x-4 text-sm text-muted-foreground mt-1">
                          {Object.entries(typeCounts).map(([type, count]) => (
                            <div key={type} className="flex items-center space-x-1">
                              {getBackendIcon(type)}
                              <span>
                                {count} {type.toUpperCase()}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                    <Badge>{backendContexts.length} backends</Badge>
                  </div>
                </CardHeader>
              </CollapsibleTrigger>

              <CollapsibleContent>
                <CardContent className="pt-0">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        {BACKEND_TABLE_HEADERS.map((header) => (
                          <TableHead
                            key={header}
                            className={header === "Actions" ? "text-right" : ""}
                          >
                            {header}
                          </TableHead>
                        ))}
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {backendContexts.map((backendContext, index) => {
                        const type = getBackendType(backendContext.backend);
                        return (
                          <TableRow key={index}>
                            <TableCell className="font-medium">
                              {getBackendName(backendContext.backend)}
                            </TableCell>
                            <TableCell>
                              <Badge
                                variant="secondary"
                                className={`${getBackendTypeColor(type)} text-white`}
                              >
                                {getBackendIcon(type)}
                                <span className="ml-1 capitalize">{type}</span>
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <Badge variant="outline">
                                {backendContext.listener.name || "unnamed"}
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <Badge variant="outline">
                                {backendContext.route.name ||
                                  `Route ${backendContext.routeIndex + 1}`}
                              </Badge>
                            </TableCell>
                            <TableCell className="text-sm text-muted-foreground">
                              {(() => {
                                const details = getBackendDetails(backendContext.backend);
                                const hasPolicies = hasBackendPolicies(backendContext.route);
                                const policyTypes = hasPolicies
                                  ? getBackendPolicyTypes(backendContext.route)
                                  : [];

                                return (
                                  <div className="space-y-1">
                                    <div>{details.primary}</div>
                                    {details.secondary && (
                                      <div className="text-xs text-muted-foreground/80 font-mono">
                                        {details.secondary}
                                      </div>
                                    )}
                                    {hasPolicies && (
                                      <div className="flex items-center space-x-1 mt-1">
                                        <Shield className="h-3 w-3 text-primary" />
                                        <span className="text-xs text-primary font-medium">
                                          Backend Policies: {policyTypes.join(", ")}
                                        </span>
                                      </div>
                                    )}
                                  </div>
                                );
                              })()}
                            </TableCell>
                            <TableCell>
                              <Badge>{backendContext.backend.weight || 1}</Badge>
                            </TableCell>
                            <TableCell className="text-right">
                              <div className="flex justify-end space-x-2">
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  onClick={() => onEditBackend(backendContext)}
                                  disabled={xds}
                                  className={xds ? "opacity-50 cursor-not-allowed" : undefined}
                                >
                                  <Edit className="h-4 w-4" />
                                </Button>
                                {(() => {
                                  // Check if deletion is allowed
                                  const totalBackendsInRoute = backendContexts.filter(
                                    (bc) =>
                                      bc.bind.port === backendContext.bind.port &&
                                      bc.listener.name === backendContext.listener.name &&
                                      bc.routeIndex === backendContext.routeIndex
                                  ).length;

                                  const deleteCheck = canDeleteBackend(
                                    backendContext.route,
                                    totalBackendsInRoute
                                  );

                                  if (!deleteCheck.canDelete) {
                                    return (
                                      <TooltipProvider>
                                        <Tooltip>
                                          <TooltipTrigger asChild>
                                            <div>
                                              <Button
                                                variant="ghost"
                                                size="icon"
                                                disabled={true}
                                                className="text-muted-foreground cursor-not-allowed"
                                              >
                                                <div className="relative">
                                                  <Trash2 className="h-4 w-4" />
                                                  <AlertTriangle className="h-2 w-2 absolute -top-0.5 -right-0.5 text-amber-500" />
                                                </div>
                                              </Button>
                                            </div>
                                          </TooltipTrigger>
                                          <TooltipContent className="max-w-sm">
                                            <p>{deleteCheck.reason}</p>
                                          </TooltipContent>
                                        </Tooltip>
                                      </TooltipProvider>
                                    );
                                  }

                                  return (
                                    <Button
                                      variant="ghost"
                                      size="icon"
                                      onClick={() => onDeleteBackend(backendContext)}
                                      className={`text-destructive hover:text-destructive ${xds ? "opacity-50 cursor-not-allowed" : ""}`}
                                      disabled={isSubmitting || xds}
                                    >
                                      <Trash2 className="h-4 w-4" />
                                    </Button>
                                  );
                                })()}
                              </div>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                </CardContent>
              </CollapsibleContent>
            </Collapsible>
          </Card>
        );
      })}
    </div>
  );
};

interface AddBackendDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  binds: Bind[];
  backendForm: typeof DEFAULT_BACKEND_FORM;
  setBackendForm: React.Dispatch<React.SetStateAction<typeof DEFAULT_BACKEND_FORM>>;
  selectedBackendType: string;
  setSelectedBackendType: React.Dispatch<React.SetStateAction<string>>;
  editingBackend: BackendWithContext | null;
  onAddBackend: () => void;
  onCancel: () => void;
  isSubmitting: boolean;
  // MCP target management
  addMcpTarget: () => void;
  removeMcpTarget: (index: number) => void;
  updateMcpTarget: (index: number, field: string, value: any) => void;
  parseAndUpdateUrl: (index: number, url: string) => void;
  updateMcpStateful: (stateful: boolean) => void;
  // Security guard management
  addSecurityGuard: (type: SecurityGuardType) => void;
  removeSecurityGuard: (index: number) => void;
  updateSecurityGuardField: (index: number, field: string, value: any) => void;
}

export const AddBackendDialog: React.FC<AddBackendDialogProps> = ({
  open,
  onOpenChange,
  binds,
  backendForm,
  setBackendForm,
  selectedBackendType,
  setSelectedBackendType,
  editingBackend,
  onAddBackend,
  onCancel,
  isSubmitting,
  addMcpTarget,
  removeMcpTarget,
  updateMcpTarget,
  parseAndUpdateUrl,
  updateMcpStateful,
  addSecurityGuard,
  removeSecurityGuard,
  updateSecurityGuardField,
}) => {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {editingBackend
              ? `Edit Backend: ${getBackendName(editingBackend.backend)}`
              : `Add ${selectedBackendType.toUpperCase()} Backend`}
          </DialogTitle>
          <DialogDescription>
            {editingBackend
              ? "Update the backend configuration."
              : "Configure a new backend for your routes."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Backend Type Selection */}
          <div className="space-y-2">
            <Label>Backend Type *</Label>
            <div className="grid grid-cols-2 gap-2">
              {BACKEND_TYPES.map(({ value, label, icon }) => {
                const IconComponent = {
                  Target,
                  Brain,
                  Cloud,
                  Server,
                  Globe,
                }[icon];

                return (
                  <Button
                    key={value}
                    type="button"
                    variant={selectedBackendType === value ? "default" : "outline"}
                    onClick={() => setSelectedBackendType(value)}
                    className="justify-start"
                  >
                    <IconComponent className="mr-2 h-4 w-4" />
                    {label}
                  </Button>
                );
              })}
            </div>
          </div>

          {/* Common fields */}
          <div
            className={
              selectedBackendType === "ai" || selectedBackendType === "mcp"
                ? "space-y-4"
                : "grid grid-cols-2 gap-4"
            }
          >
            {/* Only show name input for backends that support custom names */}
            {selectedBackendType !== "mcp" && (
              <div className="space-y-2">
                <Label htmlFor="backend-name">Name *</Label>
                <Input
                  id="backend-name"
                  value={backendForm.name}
                  onChange={(e) => setBackendForm((prev) => ({ ...prev, name: e.target.value }))}
                  placeholder="Backend name"
                />
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="backend-weight">Weight</Label>
              <Input
                id="backend-weight"
                type="number"
                min="0"
                step="1"
                value={backendForm.weight}
                onChange={(e) => setBackendForm((prev) => ({ ...prev, weight: e.target.value }))}
                placeholder="1"
              />
              <p className="text-xs text-muted-foreground">
                Weight determines load balancing priority. Higher values get more traffic.
              </p>
            </div>
          </div>

          {/* Route Selection */}
          <div className="space-y-2">
            <Label>Route *</Label>
            {editingBackend ? (
              <div className="p-3 bg-muted rounded-md">
                <p className="text-sm">
                  Port {editingBackend.bind.port} → {editingBackend.listener.name || "unnamed"} →{" "}
                  {editingBackend.route.name || `Route ${editingBackend.routeIndex + 1}`}
                </p>
                <p className="text-xs text-muted-foreground">
                  Route cannot be changed when editing
                </p>
              </div>
            ) : (
              <Select
                value={`${backendForm.selectedBindPort}|${backendForm.selectedListenerName}|${backendForm.selectedRouteIndex}`}
                onValueChange={(value) => {
                  const parts = value.split("|");
                  if (parts.length >= 3) {
                    const bindPort = parts[0];
                    const routeIndex = parts[parts.length - 1];
                    const listenerName = parts.slice(1, -1).join("|");
                    setBackendForm((prev) => ({
                      ...prev,
                      selectedBindPort: bindPort,
                      selectedListenerName: listenerName,
                      selectedRouteIndex: routeIndex,
                    }));
                  }
                }}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select a route" />
                </SelectTrigger>
                <SelectContent>
                  {getAvailableRoutes(binds).length === 0 ? (
                    <div className="py-2 px-3 text-sm text-muted-foreground">
                      No routes available. Create a route first.
                    </div>
                  ) : (
                    getAvailableRoutes(binds).map((route) => (
                      <SelectItem
                        key={`${route.bindPort}|${route.listenerName}|${route.routeIndex}`}
                        value={`${route.bindPort}|${route.listenerName}|${route.routeIndex}`}
                      >
                        Port {route.bindPort} → {route.listenerName} → {route.routeName} (
                        {route.path})
                      </SelectItem>
                    ))
                  )}
                </SelectContent>
              </Select>
            )}
          </div>

          {/* Service Backend Configuration */}
          {selectedBackendType === "service" && (
            <ServiceBackendForm backendForm={backendForm} setBackendForm={setBackendForm} />
          )}

          {/* Host Backend Configuration */}
          {selectedBackendType === "host" && (
            <HostBackendForm backendForm={backendForm} setBackendForm={setBackendForm} />
          )}

          {/* MCP Backend Configuration */}
          {selectedBackendType === "mcp" && (
            <McpBackendForm
              backendForm={backendForm}
              addMcpTarget={addMcpTarget}
              removeMcpTarget={removeMcpTarget}
              updateMcpTarget={updateMcpTarget}
              parseAndUpdateUrl={parseAndUpdateUrl}
              updateMcpStateful={updateMcpStateful}
              addSecurityGuard={addSecurityGuard}
              removeSecurityGuard={removeSecurityGuard}
              updateSecurityGuardField={updateSecurityGuardField}
            />
          )}

          {/* AI Backend Configuration */}
          {selectedBackendType === "ai" && (
            <AiBackendForm backendForm={backendForm} setBackendForm={setBackendForm} />
          )}

          {/* Dynamic Backend Configuration */}
          {selectedBackendType === "dynamic" && (
            <div className="p-4 bg-muted/50 rounded-lg">
              <div className="flex items-center space-x-2 mb-2">
                <Globe className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">Dynamic Backend</span>
              </div>
              <p className="text-sm text-muted-foreground">
                Dynamic backends are automatically configured and don&apos;t require additional
                settings.
              </p>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button onClick={onAddBackend} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {editingBackend ? "Update" : "Add"} {selectedBackendType.toUpperCase()} Backend
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

// Service Backend Form Component
interface ServiceBackendFormProps {
  backendForm: typeof DEFAULT_BACKEND_FORM;
  setBackendForm: React.Dispatch<React.SetStateAction<typeof DEFAULT_BACKEND_FORM>>;
}

const ServiceBackendForm: React.FC<ServiceBackendFormProps> = ({ backendForm, setBackendForm }) => (
  <div className="space-y-4">
    <div className="grid grid-cols-2 gap-4">
      <div className="space-y-2">
        <Label htmlFor="service-namespace">Namespace *</Label>
        <Input
          id="service-namespace"
          value={backendForm.serviceNamespace}
          onChange={(e) =>
            setBackendForm((prev) => ({ ...prev, serviceNamespace: e.target.value }))
          }
          placeholder="default"
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="service-hostname">Hostname *</Label>
        <Input
          id="service-hostname"
          value={backendForm.serviceHostname}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, serviceHostname: e.target.value }))}
          placeholder="my-service"
        />
      </div>
    </div>
    <div className="space-y-2">
      <Label htmlFor="service-port">Port *</Label>
      <Input
        id="service-port"
        type="number"
        min="0"
        max="65535"
        value={backendForm.servicePort}
        onChange={(e) => setBackendForm((prev) => ({ ...prev, servicePort: e.target.value }))}
        placeholder="80"
      />
    </div>
  </div>
);

// Host Backend Form Component
interface HostBackendFormProps {
  backendForm: typeof DEFAULT_BACKEND_FORM;
  setBackendForm: React.Dispatch<React.SetStateAction<typeof DEFAULT_BACKEND_FORM>>;
}

const HostBackendForm: React.FC<HostBackendFormProps> = ({ backendForm, setBackendForm }) => (
  <div className="space-y-4">
    <div className="space-y-2">
      <Label>Host Type *</Label>
      <div className="flex space-x-4">
        {HOST_TYPES.map(({ value, label }) => (
          <Button
            key={value}
            type="button"
            variant={backendForm.hostType === value ? "default" : "outline"}
            onClick={() => setBackendForm((prev) => ({ ...prev, hostType: value as any }))}
          >
            {label}
          </Button>
        ))}
      </div>
    </div>

    {backendForm.hostType === "address" ? (
      <div className="space-y-2">
        <Label htmlFor="host-address">Address *</Label>
        <Input
          id="host-address"
          value={backendForm.hostAddress}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, hostAddress: e.target.value }))}
          placeholder="192.168.1.100:8080"
        />
      </div>
    ) : (
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="host-hostname">Hostname *</Label>
          <Input
            id="host-hostname"
            value={backendForm.hostHostname}
            onChange={(e) => setBackendForm((prev) => ({ ...prev, hostHostname: e.target.value }))}
            placeholder="example.com"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="host-port">Port *</Label>
          <Input
            id="host-port"
            type="number"
            min="0"
            max="65535"
            value={backendForm.hostPort}
            onChange={(e) => setBackendForm((prev) => ({ ...prev, hostPort: e.target.value }))}
            placeholder="8080"
          />
        </div>
      </div>
    )}
  </div>
);

// Security Guard icon mapping
const getSecurityGuardIcon = (type: SecurityGuardType) => {
  switch (type) {
    case "pii":
      return <Eye className="h-4 w-4" />;
    case "tool_poisoning":
      return <ShieldAlert className="h-4 w-4" />;
    case "rug_pull":
      return <AlertTriangle className="h-4 w-4" />;
    case "tool_shadowing":
      return <Copy className="h-4 w-4" />;
    case "server_whitelist":
      return <CheckCircle className="h-4 w-4" />;
    case "wasm":
      return <Code className="h-4 w-4" />;
    default:
      return <Shield className="h-4 w-4" />;
  }
};

// Security Guards Section Component
interface SecurityGuardsSectionProps {
  guards: SecurityGuard[];
  addSecurityGuard: (type: SecurityGuardType) => void;
  removeSecurityGuard: (index: number) => void;
  updateSecurityGuardField: (index: number, field: string, value: any) => void;
}

const SecurityGuardsSection: React.FC<SecurityGuardsSectionProps> = ({
  guards,
  addSecurityGuard,
  removeSecurityGuard,
  updateSecurityGuardField,
}) => {
  const [isExpanded, setIsExpanded] = React.useState(guards.length > 0);
  const [expandedGuards, setExpandedGuards] = React.useState<Set<number>>(new Set());
  const { schemas } = useGuardSchemas();

  const toggleGuardExpanded = (index: number) => {
    setExpandedGuards((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <div className="flex items-center justify-between py-2">
        <CollapsibleTrigger asChild>
          <Button variant="ghost" size="sm" className="p-0 hover:bg-transparent">
            {isExpanded ? (
              <ChevronDown className="h-4 w-4 mr-2" />
            ) : (
              <ChevronRight className="h-4 w-4 mr-2" />
            )}
            <Shield className="h-4 w-4 mr-2" />
            <span className="font-medium">Security Guards</span>
            {guards.length > 0 && (
              <Badge variant="secondary" className="ml-2">
                {guards.length}
              </Badge>
            )}
          </Button>
        </CollapsibleTrigger>
        <Select onValueChange={(value) => addSecurityGuard(value as SecurityGuardType)}>
          <SelectTrigger className="w-[160px]">
            <Plus className="h-3 w-3 mr-1" />
            <span>Add Guard</span>
          </SelectTrigger>
          <SelectContent>
            {SECURITY_GUARD_TYPES.map(({ value, label }) => (
              <SelectItem key={value} value={value}>
                <div className="flex items-center gap-2">
                  {getSecurityGuardIcon(value)}
                  <span>{label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <CollapsibleContent className="space-y-3">
        {guards.length === 0 ? (
          <div className="text-center py-6 border-2 border-dashed border-muted rounded-md">
            <Shield className="mx-auto h-8 w-8 text-muted-foreground mb-2" />
            <p className="text-sm text-muted-foreground">No security guards configured</p>
            <p className="text-xs text-muted-foreground">
              Add guards to protect MCP communications
            </p>
          </div>
        ) : (
          guards.map((guard, index) => (
            <Card key={index} className="p-3">
              <div className="space-y-3">
                {/* Guard Header */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="p-0 h-auto"
                      onClick={() => toggleGuardExpanded(index)}
                    >
                      {expandedGuards.has(index) ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                    </Button>
                    <Badge variant="outline" className="flex items-center gap-1">
                      {getSecurityGuardIcon(guard.type)}
                      {SECURITY_GUARD_TYPES.find((t) => t.value === guard.type)?.label ||
                        guard.type}
                    </Badge>
                    <span className="text-sm text-muted-foreground">{guard.id}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <label className="flex items-center gap-1 text-sm cursor-pointer">
                      <input
                        type="checkbox"
                        checked={guard.enabled}
                        onChange={(e) =>
                          updateSecurityGuardField(index, "enabled", e.target.checked)
                        }
                        className="form-checkbox h-4 w-4"
                      />
                      Enabled
                    </label>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => removeSecurityGuard(index)}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                {/* Guard Configuration (expandable) */}
                {expandedGuards.has(index) && (
                  <div className="space-y-4 pt-2 border-t">
                    {/* Common Fields */}
                    <div className="grid grid-cols-2 gap-3">
                      <div className="space-y-1">
                        <Label className="text-xs">Guard ID *</Label>
                        <Input
                          value={guard.id}
                          onChange={(e) => updateSecurityGuardField(index, "id", e.target.value)}
                          placeholder="my-guard"
                          className="h-8 text-sm"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Priority (0-100)</Label>
                        <Input
                          type="number"
                          min={0}
                          max={100}
                          value={guard.priority}
                          onChange={(e) =>
                            updateSecurityGuardField(
                              index,
                              "priority",
                              parseInt(e.target.value) || 0
                            )
                          }
                          className="h-8 text-sm"
                        />
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-3">
                      <div className="space-y-1">
                        <Label className="text-xs">Timeout (ms)</Label>
                        <Input
                          type="number"
                          min={10}
                          max={10000}
                          value={guard.timeout_ms}
                          onChange={(e) =>
                            updateSecurityGuardField(
                              index,
                              "timeout_ms",
                              parseInt(e.target.value) || 100
                            )
                          }
                          className="h-8 text-sm"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Failure Mode</Label>
                        <Select
                          value={guard.failure_mode}
                          onValueChange={(value) =>
                            updateSecurityGuardField(index, "failure_mode", value)
                          }
                        >
                          <SelectTrigger className="h-8 text-sm">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {FAILURE_MODES.map(({ value, label }) => (
                              <SelectItem key={value} value={value}>
                                {label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    </div>

                    {/* Runs On (multi-select as checkboxes) */}
                    <div className="space-y-1">
                      <Label className="text-xs">Runs On *</Label>
                      <div className="flex flex-wrap gap-3">
                        {GUARD_PHASES.map(({ value, label }) => (
                          <label
                            key={value}
                            className="flex items-center gap-1 text-sm cursor-pointer"
                          >
                            <input
                              type="checkbox"
                              checked={guard.runs_on.includes(value)}
                              onChange={(e) => {
                                const newRunsOn = e.target.checked
                                  ? [...guard.runs_on, value]
                                  : guard.runs_on.filter((p) => p !== value);
                                updateSecurityGuardField(index, "runs_on", newRunsOn);
                              }}
                              className="form-checkbox h-4 w-4"
                            />
                            {label}
                          </label>
                        ))}
                      </div>
                    </div>

                    {/* Type-specific configuration */}
                    {guard.type === "pii" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          PII Detection Settings
                        </h5>
                        <div className="space-y-1">
                          <Label className="text-xs">Detect PII Types *</Label>
                          <div className="flex flex-wrap gap-2">
                            {PII_TYPES.map(({ value, label }) => (
                              <label
                                key={value}
                                className="flex items-center gap-1 text-sm cursor-pointer"
                              >
                                <input
                                  type="checkbox"
                                  checked={guard.detect.includes(value)}
                                  onChange={(e) => {
                                    const newDetect = e.target.checked
                                      ? [...guard.detect, value]
                                      : guard.detect.filter((t) => t !== value);
                                    updateSecurityGuardField(index, "detect", newDetect);
                                  }}
                                  className="form-checkbox h-4 w-4"
                                />
                                {label}
                              </label>
                            ))}
                          </div>
                        </div>
                        <div className="grid grid-cols-2 gap-3">
                          <div className="space-y-1">
                            <Label className="text-xs">Action</Label>
                            <Select
                              value={guard.action}
                              onValueChange={(value) =>
                                updateSecurityGuardField(index, "action", value)
                              }
                            >
                              <SelectTrigger className="h-8 text-sm">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                {PII_ACTIONS.map(({ value, label }) => (
                                  <SelectItem key={value} value={value}>
                                    {label}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </div>
                          <div className="space-y-1">
                            <Label className="text-xs">Min Score (0-1)</Label>
                            <Input
                              type="number"
                              min={0}
                              max={1}
                              step={0.1}
                              value={guard.min_score}
                              onChange={(e) =>
                                updateSecurityGuardField(
                                  index,
                                  "min_score",
                                  parseFloat(e.target.value) || 0.3
                                )
                              }
                              className="h-8 text-sm"
                            />
                          </div>
                        </div>
                        {guard.action === "reject" && (
                          <div className="space-y-1">
                            <Label className="text-xs">Rejection Message</Label>
                            <Input
                              value={guard.rejection_message || ""}
                              onChange={(e) =>
                                updateSecurityGuardField(index, "rejection_message", e.target.value)
                              }
                              placeholder="PII detected - request rejected"
                              className="h-8 text-sm"
                            />
                          </div>
                        )}
                      </div>
                    )}

                    {guard.type === "tool_poisoning" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          Tool Poisoning Settings
                        </h5>
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={guard.strict_mode}
                            onChange={(e) =>
                              updateSecurityGuardField(index, "strict_mode", e.target.checked)
                            }
                            className="form-checkbox h-4 w-4"
                          />
                          <Label className="text-sm cursor-pointer">Strict Mode</Label>
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs">Scan Fields</Label>
                          <div className="flex flex-wrap gap-3">
                            {SCAN_FIELDS.map(({ value, label }) => (
                              <label
                                key={value}
                                className="flex items-center gap-1 text-sm cursor-pointer"
                              >
                                <input
                                  type="checkbox"
                                  checked={guard.scan_fields.includes(value)}
                                  onChange={(e) => {
                                    const newFields = e.target.checked
                                      ? [...guard.scan_fields, value]
                                      : guard.scan_fields.filter((f) => f !== value);
                                    updateSecurityGuardField(index, "scan_fields", newFields);
                                  }}
                                  className="form-checkbox h-4 w-4"
                                />
                                {label}
                              </label>
                            ))}
                          </div>
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs">Alert Threshold</Label>
                          <Input
                            type="number"
                            min={1}
                            value={guard.alert_threshold}
                            onChange={(e) =>
                              updateSecurityGuardField(
                                index,
                                "alert_threshold",
                                parseInt(e.target.value) || 1
                              )
                            }
                            className="h-8 text-sm w-24"
                          />
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs">Custom Patterns (one per line)</Label>
                          <textarea
                            value={guard.custom_patterns.join("\n")}
                            onChange={(e) =>
                              updateSecurityGuardField(
                                index,
                                "custom_patterns",
                                e.target.value.split("\n").filter(Boolean)
                              )
                            }
                            placeholder="(?i)SYSTEM:\s*override&#10;(?i)ignore\s+all\s+previous"
                            className="w-full h-20 px-2 py-1 text-sm border rounded-md"
                          />
                        </div>
                      </div>
                    )}

                    {guard.type === "rug_pull" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          Rug Pull Settings
                        </h5>
                        <div className="space-y-1">
                          <Label className="text-xs">Risk Threshold</Label>
                          <Input
                            type="number"
                            min={1}
                            value={guard.risk_threshold}
                            onChange={(e) =>
                              updateSecurityGuardField(
                                index,
                                "risk_threshold",
                                parseInt(e.target.value) || 5
                              )
                            }
                            className="h-8 text-sm w-24"
                          />
                        </div>
                      </div>
                    )}

                    {guard.type === "tool_shadowing" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          Tool Shadowing Settings
                        </h5>
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={guard.block_duplicates}
                            onChange={(e) =>
                              updateSecurityGuardField(index, "block_duplicates", e.target.checked)
                            }
                            className="form-checkbox h-4 w-4"
                          />
                          <Label className="text-sm cursor-pointer">Block Duplicates</Label>
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs">Protected Names (one per line)</Label>
                          <textarea
                            value={guard.protected_names.join("\n")}
                            onChange={(e) =>
                              updateSecurityGuardField(
                                index,
                                "protected_names",
                                e.target.value.split("\n").filter(Boolean)
                              )
                            }
                            placeholder="initialize&#10;tools/list&#10;tools/call"
                            className="w-full h-20 px-2 py-1 text-sm border rounded-md"
                          />
                        </div>
                      </div>
                    )}

                    {guard.type === "server_whitelist" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          Server Whitelist Settings
                        </h5>
                        <div className="space-y-1">
                          <Label className="text-xs">Allowed Servers (one per line)</Label>
                          <textarea
                            value={guard.allowed_servers.join("\n")}
                            onChange={(e) =>
                              updateSecurityGuardField(
                                index,
                                "allowed_servers",
                                e.target.value.split("\n").filter(Boolean)
                              )
                            }
                            placeholder="github-mcp&#10;slack-mcp"
                            className="w-full h-20 px-2 py-1 text-sm border rounded-md"
                          />
                        </div>
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={guard.detect_typosquats}
                            onChange={(e) =>
                              updateSecurityGuardField(index, "detect_typosquats", e.target.checked)
                            }
                            className="form-checkbox h-4 w-4"
                          />
                          <Label className="text-sm cursor-pointer">Detect Typosquats</Label>
                        </div>
                        {guard.detect_typosquats && (
                          <div className="space-y-1">
                            <Label className="text-xs">Similarity Threshold (0-1)</Label>
                            <Input
                              type="number"
                              min={0}
                              max={1}
                              step={0.05}
                              value={guard.similarity_threshold}
                              onChange={(e) =>
                                updateSecurityGuardField(
                                  index,
                                  "similarity_threshold",
                                  parseFloat(e.target.value) || 0.85
                                )
                              }
                              className="h-8 text-sm w-24"
                            />
                          </div>
                        )}
                      </div>
                    )}

                    {guard.type === "wasm" && (
                      <div className="space-y-3 p-3 bg-muted/50 rounded-md">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          WASM Guard Settings
                        </h5>
                        <div className="space-y-1">
                          <Label className="text-xs">Module Path *</Label>
                          <Input
                            value={guard.module_path}
                            onChange={(e) =>
                              updateSecurityGuardField(index, "module_path", e.target.value)
                            }
                            placeholder="./path/to/guard.wasm"
                            className="h-8 text-sm"
                          />
                          {guard.module_path && !guard.module_path.endsWith(".wasm") && (
                            <p className="text-xs text-yellow-600">
                              Warning: Path should end with .wasm
                            </p>
                          )}
                        </div>
                        <div className="grid grid-cols-2 gap-3">
                          <div className="space-y-1">
                            <Label className="text-xs">Max Memory (MB)</Label>
                            <Input
                              type="number"
                              min={1}
                              max={1024}
                              value={Math.round(guard.max_memory / (1024 * 1024))}
                              onChange={(e) =>
                                updateSecurityGuardField(
                                  index,
                                  "max_memory",
                                  (parseInt(e.target.value) || 10) * 1024 * 1024
                                )
                              }
                              className="h-8 text-sm"
                            />
                          </div>
                          <div className="space-y-1">
                            <Label className="text-xs">Max WASM Stack (MB)</Label>
                            <Input
                              type="number"
                              min={1}
                              max={64}
                              step={1}
                              value={Math.round(guard.max_wasm_stack / (1024 * 1024))}
                              onChange={(e) =>
                                updateSecurityGuardField(
                                  index,
                                  "max_wasm_stack",
                                  (parseInt(e.target.value) || 2) * 1024 * 1024
                                )
                              }
                              className="h-8 text-sm"
                            />
                          </div>
                        </div>
                        {/* Schema-driven config form or JSON fallback */}
                        {(() => {
                          // For WASM guards, match schema by module_path filename or guardType
                          const guardSchema = (() => {
                            // Direct match by guard id
                            if (schemas[guard.id]) return schemas[guard.id];

                            if (guard.type === "wasm" && guard.module_path) {
                              // Extract base name: "server_spoofing_guard.wasm" -> "server_spoofing"
                              const filename =
                                guard.module_path
                                  .split("/")
                                  .pop()
                                  ?.replace(/_guard\.wasm$/, "")
                                  .replace(/\.wasm$/, "") || "";

                              // Match against x-guard-meta.guardType
                              const byGuardType = Object.entries(schemas).find(([key, s]) => {
                                const gt = s["x-guard-meta"]?.guardType;
                                return gt === filename || key === filename;
                              });
                              if (byGuardType) return byGuardType[1];
                            }

                            // Fallback: match by guardType === guard.id
                            return Object.values(schemas).find(
                              (s) => s["x-guard-meta"]?.guardType === guard.id
                            );
                          })();

                          if (guardSchema) {
                            return (
                              <SchemaForm
                                schema={guardSchema}
                                value={guard.config || {}}
                                onChange={(newConfig) =>
                                  updateSecurityGuardField(index, "config", newConfig)
                                }
                              />
                            );
                          }

                          return (
                            <div className="space-y-1">
                              <Label className="text-xs">Config (JSON)</Label>
                              <textarea
                                value={JSON.stringify(guard.config || {}, null, 2)}
                                onChange={(e) => {
                                  try {
                                    updateSecurityGuardField(
                                      index,
                                      "config",
                                      JSON.parse(e.target.value)
                                    );
                                  } catch {
                                    /* ignore parse errors while typing */
                                  }
                                }}
                                placeholder="{}"
                                className="w-full h-32 px-2 py-1 text-sm border rounded-md font-mono"
                              />
                            </div>
                          );
                        })()}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </Card>
          ))
        )}
      </CollapsibleContent>
    </Collapsible>
  );
};

// MCP Backend Form Component
interface McpBackendFormProps {
  backendForm: typeof DEFAULT_BACKEND_FORM;
  addMcpTarget: () => void;
  removeMcpTarget: (index: number) => void;
  updateMcpTarget: (index: number, field: string, value: any) => void;
  parseAndUpdateUrl: (index: number, url: string) => void;
  updateMcpStateful: (stateful: boolean) => void;
  addSecurityGuard: (type: SecurityGuardType) => void;
  removeSecurityGuard: (index: number) => void;
  updateSecurityGuardField: (index: number, field: string, value: any) => void;
}

const McpBackendForm: React.FC<McpBackendFormProps> = ({
  backendForm,
  addMcpTarget,
  removeMcpTarget,
  updateMcpTarget,
  parseAndUpdateUrl,
  updateMcpStateful,
  addSecurityGuard,
  removeSecurityGuard,
  updateSecurityGuardField,
}) => (
  <div className="space-y-4">
    <div className="flex items-center justify-between">
      <Label>MCP Targets</Label>
      <Button type="button" variant="outline" size="sm" onClick={addMcpTarget}>
        <Plus className="mr-1 h-3 w-3" />
        Add Target
      </Button>
    </div>

    {backendForm.mcpTargets.map((target, index) => (
      <Card key={index} className="p-4">
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium">Target {index + 1}</h4>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => removeMcpTarget(index)}
              className="text-destructive hover:text-destructive"
            >
              <Trash2 className="h-3 w-3" />
            </Button>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Target Name *</Label>
              <Input
                value={target.name}
                onChange={(e) => updateMcpTarget(index, "name", e.target.value)}
                placeholder="my-target"
              />
            </div>
            <div className="space-y-2">
              <Label>Target Type *</Label>
              <Select
                value={target.type}
                onValueChange={(value) => updateMcpTarget(index, "type", value)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {MCP_TARGET_TYPES.map(({ value, label }) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {(target.type === "sse" || target.type === "mcp" || target.type === "openapi") && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label>URL *</Label>
                <Input
                  value={target.fullUrl}
                  onChange={(e) => parseAndUpdateUrl(index, e.target.value)}
                  placeholder="https://example.com/mcp"
                />
              </div>
            </div>
          )}

          {target.type === "stdio" && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label>Command *</Label>
                <Input
                  value={target.cmd}
                  onChange={(e) => updateMcpTarget(index, "cmd", e.target.value)}
                  placeholder="python3 my_mcp_server.py"
                />
              </div>
              {/* Arguments Section */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label>Arguments</Label>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      const currentArgs = Array.isArray(target.args) ? target.args : [];
                      updateMcpTarget(index, "args", [...currentArgs, ""]);
                    }}
                  >
                    <Plus className="mr-1 h-3 w-3" />
                    Add Argument
                  </Button>
                </div>
                {Array.isArray(target.args) && target.args.length > 0 ? (
                  <div className="space-y-2">
                    {target.args.map((arg, argIndex) => (
                      <div key={argIndex} className="flex items-center space-x-2">
                        <Input
                          value={arg}
                          onChange={(e) => {
                            const newArgs = [...target.args];
                            newArgs[argIndex] = e.target.value;
                            updateMcpTarget(index, "args", newArgs);
                          }}
                          placeholder="--verbose"
                          className="flex-1"
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            const newArgs = Array.isArray(target.args)
                              ? target.args.filter((_, i: number) => i !== argIndex)
                              : [];
                            updateMcpTarget(index, "args", newArgs);
                          }}
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-4 border-2 border-dashed border-muted rounded-md">
                    <p className="text-sm text-muted-foreground">No arguments configured</p>
                  </div>
                )}
              </div>

              {/* Environment Variables Section */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label>Environment Variables</Label>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      const currentEnv = getEnvAsRecord(target.env);
                      updateMcpTarget(index, "env", { ...currentEnv, "": "" });
                    }}
                  >
                    <Plus className="mr-1 h-3 w-3" />
                    Add Variable
                  </Button>
                </div>
                {Object.keys(getEnvAsRecord(target.env)).length > 0 ? (
                  <div className="space-y-2">
                    {Object.entries(getEnvAsRecord(target.env)).map(([key, value], envIndex) => (
                      <div key={envIndex} className="flex items-center space-x-2">
                        <Input
                          value={key}
                          onChange={(e) => {
                            const currentEnv = getEnvAsRecord(target.env);
                            const newEnv = { ...currentEnv };
                            delete newEnv[key];
                            newEnv[e.target.value] = String(value);
                            updateMcpTarget(index, "env", newEnv);
                          }}
                          placeholder="DEBUG"
                          className="flex-1"
                        />
                        <span className="text-muted-foreground">=</span>
                        <Input
                          value={String(value)}
                          onChange={(e) => {
                            const currentEnv = getEnvAsRecord(target.env);
                            const newEnv = { ...currentEnv };
                            newEnv[key] = e.target.value;
                            updateMcpTarget(index, "env", newEnv);
                          }}
                          placeholder="true"
                          className="flex-1"
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            const currentEnv = getEnvAsRecord(target.env);
                            const newEnv = { ...currentEnv };
                            delete newEnv[key];
                            updateMcpTarget(index, "env", newEnv);
                          }}
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-4 border-2 border-dashed border-muted rounded-md">
                    <p className="text-sm text-muted-foreground">
                      No environment variables configured
                    </p>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </Card>
    ))}

    {backendForm.mcpTargets.length === 0 && (
      <div className="text-center py-8 border-2 border-dashed border-muted rounded-md">
        <Target className="mx-auto h-8 w-8 text-muted-foreground mb-2" />
        <p className="text-sm text-muted-foreground">No targets configured</p>
        <p className="text-xs text-muted-foreground">
          Add at least one target to create an MCP backend
        </p>
      </div>
    )}

    {/* Security Guards Section */}
    <SecurityGuardsSection
      guards={backendForm.securityGuards}
      addSecurityGuard={addSecurityGuard}
      removeSecurityGuard={removeSecurityGuard}
      updateSecurityGuardField={updateSecurityGuardField}
    />

    <div className="flex items-center space-x-2">
      <input
        type="checkbox"
        id="mcp-stateful"
        checked={!!backendForm.mcpStateful}
        onChange={(e) => updateMcpStateful(e.target.checked)}
        className="form-checkbox h-4 w-4"
      />
      <Label htmlFor="mcp-stateful" className="cursor-pointer">
        Enable stateful mode
      </Label>
    </div>
  </div>
);

// AI Backend Form Component
interface AiBackendFormProps {
  backendForm: typeof DEFAULT_BACKEND_FORM;
  setBackendForm: React.Dispatch<React.SetStateAction<typeof DEFAULT_BACKEND_FORM>>;
}

const AiBackendForm: React.FC<AiBackendFormProps> = ({ backendForm, setBackendForm }) => (
  <div className="space-y-4">
    <div className="space-y-2">
      <Label>AI Provider *</Label>
      <div className="grid grid-cols-3 gap-2">
        {AI_PROVIDERS.map(({ value, label }) => (
          <Button
            key={value}
            type="button"
            variant={backendForm.aiProvider === value ? "default" : "outline"}
            onClick={() => setBackendForm((prev) => ({ ...prev, aiProvider: value as any }))}
            className="text-sm"
          >
            {label}
          </Button>
        ))}
      </div>
    </div>

    <div className="grid grid-cols-2 gap-4">
      <div className="space-y-2">
        <Label htmlFor="ai-model">
          Model {backendForm.aiProvider === "bedrock" ? "*" : "(optional)"}
        </Label>
        <Input
          id="ai-model"
          value={backendForm.aiModel}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, aiModel: e.target.value }))}
          placeholder={AI_MODEL_PLACEHOLDERS[backendForm.aiProvider]}
        />
      </div>

      {(backendForm.aiProvider === "vertex" || backendForm.aiProvider === "bedrock") && (
        <div className="space-y-2">
          <Label htmlFor="ai-region">
            Region {backendForm.aiProvider === "bedrock" ? "*" : "(optional)"}
          </Label>
          <Input
            id="ai-region"
            value={backendForm.aiRegion}
            onChange={(e) => setBackendForm((prev) => ({ ...prev, aiRegion: e.target.value }))}
            placeholder={AI_REGION_PLACEHOLDERS[backendForm.aiProvider]}
          />
        </div>
      )}
    </div>

    {backendForm.aiProvider === "vertex" && (
      <div className="space-y-2">
        <Label htmlFor="ai-project-id">Project ID *</Label>
        <Input
          id="ai-project-id"
          value={backendForm.aiProjectId}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, aiProjectId: e.target.value }))}
          placeholder="my-gcp-project"
        />
      </div>
    )}

    {backendForm.aiProvider === "azureOpenAI" && (
      <div className="space-y-2">
        <Label htmlFor="ai-host">Host *</Label>
        <Input
          id="ai-host"
          value={backendForm.aiHost}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, aiHost: e.target.value }))}
          placeholder="my-resource-name.openai.azure.com"
        />
      </div>
    )}
    {backendForm.aiProvider === "azureOpenAI" && (
      <div className="space-y-2">
        <Label htmlFor="ai-api-version">API Version (optional)</Label>
        <Input
          id="ai-api-version"
          value={backendForm.aiApiVersion}
          onChange={(e) => setBackendForm((prev) => ({ ...prev, aiApiVersion: e.target.value }))}
          placeholder="v1, preview, 2024-10-21, etc. (defaults to v1)"
        />
      </div>
    )}

    {/* AI Host Override */}
    <div className="space-y-2">
      <Label htmlFor="ai-host-override">Host Override (optional)</Label>
      <Input
        id="ai-host-override"
        value={backendForm.aiHostOverride}
        onChange={(e) => setBackendForm((prev) => ({ ...prev, aiHostOverride: e.target.value }))}
        placeholder="api.custom-ai-provider.com:443"
      />
    </div>

    {/* AI Path Override */}
    <div className="space-y-2">
      <Label htmlFor="ai-path-override">Path Override (optional)</Label>
      <Input
        id="ai-path-override"
        value={backendForm.aiPathOverride}
        onChange={(e) => setBackendForm((prev) => ({ ...prev, aiPathOverride: e.target.value }))}
        placeholder="/v1/chat/completions"
      />
    </div>
  </div>
);
