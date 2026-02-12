"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Pencil,
  Code2,
  Rocket,
  ArrowRight,
  Settings,
  Clock,
  AlertCircle,
  Shield,
  FileCheck,
  Bug,
  GitPullRequest,
  ClipboardCheck,
} from "lucide-react";

// Phase types
type Phase = "design" | "develop" | "deploy";

// Agent status types
type AgentStatus = "active" | "idle" | "review_needed";

interface Agent {
  id: string;
  name: string;
  description: string;
  shortDescription: string;
  phase: Phase;
  status: AgentStatus;
  lastActive: string;
  findings: number;
  holdingContext?: {
    name: string;
    id: string;
  };
  capabilities: string[];
  icon: React.ReactNode;
}

// Sample data following Kamal's Synthesis design
const agents: Agent[] = [
  {
    id: "principal-security-eng",
    name: "Principal Security Eng.",
    shortDescription: "STRIDE Threat Modeling & DFDs",
    description:
      "Threat modeling using STRIDE and AWS Threat Grammar. Generates threat statements, maps trust boundaries, produces data flow diagrams, and proposes mitigations.",
    phase: "design",
    status: "active",
    lastActive: "30 min ago",
    findings: 8,
    holdingContext: {
      name: "Threat Model JSON (TM-001)",
      id: "tm-001",
    },
    capabilities: ["Source code", "Design docs", "Previous threat models"],
    icon: <Shield className="h-5 w-5" />,
  },
  {
    id: "compliance-guardian",
    name: "Compliance Guardian",
    shortDescription: "SOC2/GDPR Blueprint Validation",
    description:
      "Validates system architecture, data flows, and access patterns against SOC2 Type II controls, GDPR Art. 25/32 requirements, and industry compliance blueprints.",
    phase: "design",
    status: "review_needed",
    lastActive: "2 hours ago",
    findings: 3,
    holdingContext: {
      name: "SOC2 Gap Analysis v2.1",
      id: "soc2-v21",
    },
    capabilities: ["Compliance policies", "IaC templates", "Data flow maps"],
    icon: <FileCheck className="h-5 w-5" />,
  },
  {
    id: "vulnerability-autofix",
    name: "Vulnerability AutoFix",
    shortDescription: "Automated Security Remediation",
    description:
      "Scans code for security vulnerabilities using SAST/DAST tools and generates automated fixes with PR creation.",
    phase: "develop",
    status: "active",
    lastActive: "15 min ago",
    findings: 12,
    holdingContext: {
      name: "PR #402 AutoFix",
      id: "pr-402",
    },
    capabilities: ["Source code", "Vulnerability reports", "CI/CD configs"],
    icon: <Bug className="h-5 w-5" />,
  },
  {
    id: "code-reviewer",
    name: "Code Reviewer",
    shortDescription: "AI-Powered Code Review",
    description:
      "Reviews pull requests for security issues, code quality, and best practices. Integrates with GitHub/GitLab.",
    phase: "develop",
    status: "idle",
    lastActive: "1 hour ago",
    findings: 5,
    capabilities: ["Pull requests", "Code diffs", "Review history"],
    icon: <GitPullRequest className="h-5 w-5" />,
  },
  {
    id: "opa-gate-check",
    name: "OPA Gate Check",
    shortDescription: "Policy-as-Code Validation",
    description:
      "Validates deployments against OPA policies. Checks infrastructure-as-code, container configs, and deployment manifests.",
    phase: "deploy",
    status: "active",
    lastActive: "5 min ago",
    findings: 2,
    holdingContext: {
      name: "OPA Gate Check",
      id: "opa-001",
    },
    capabilities: ["Kubernetes manifests", "Terraform plans", "Docker configs"],
    icon: <ClipboardCheck className="h-5 w-5" />,
  },
  {
    id: "runtime-guardian",
    name: "Runtime Guardian",
    shortDescription: "Production Security Monitoring",
    description:
      "Monitors production environments for security anomalies, unauthorized access, and compliance violations.",
    phase: "deploy",
    status: "idle",
    lastActive: "3 hours ago",
    findings: 3,
    capabilities: ["Runtime logs", "Security events", "Compliance reports"],
    icon: <Shield className="h-5 w-5" />,
  },
];

// Context items for the graph
const contextItems = {
  design: [
    { name: "Threat Model TM-001", status: "complete" },
    { name: "SOC2 Gap Analysis", status: "complete" },
  ],
  develop: [{ name: "PR #402 AutoFix", status: "in_progress" }],
  deploy: [{ name: "OPA Gate Check", status: "complete" }],
};

export default function AgentPipelinePage() {
  const [activeFilter, setActiveFilter] = useState<"all" | Phase>("all");

  const getPhaseStats = (phase: Phase) => {
    const phaseAgents = agents.filter((a) => a.phase === phase);
    return {
      total: phaseAgents.length,
      active: phaseAgents.filter((a) => a.status === "active").length,
      findings: phaseAgents.reduce((sum, a) => sum + a.findings, 0),
    };
  };

  const filteredAgents =
    activeFilter === "all" ? agents : agents.filter((a) => a.phase === activeFilter);

  const getStatusBadge = (status: AgentStatus) => {
    switch (status) {
      case "active":
        return (
          <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-100">
            Active
          </Badge>
        );
      case "review_needed":
        return (
          <Badge className="bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-100">
            Review Needed
          </Badge>
        );
      default:
        return (
          <Badge variant="secondary" className="bg-gray-100 text-gray-800">
            Idle
          </Badge>
        );
    }
  };

  const getPhaseBadge = (phase: Phase) => {
    const colors = {
      design: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-100",
      develop: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-100",
      deploy: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-100",
    };
    return (
      <Badge className={colors[phase]}>{phase.charAt(0).toUpperCase() + phase.slice(1)}</Badge>
    );
  };

  const getPhaseIcon = (phase: Phase) => {
    switch (phase) {
      case "design":
        return <Pencil className="h-6 w-6" />;
      case "develop":
        return <Code2 className="h-6 w-6" />;
      case "deploy":
        return <Rocket className="h-6 w-6" />;
    }
  };

  const activeAgentsCount = agents.filter((a) => a.status === "active").length;

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-2xl font-bold">Agent Pipeline</h1>
          <Badge variant="outline" className="flex items-center gap-1">
            <span className="h-2 w-2 bg-green-500 rounded-full animate-pulse" />
            {activeAgentsCount} agents active
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            Search
          </Button>
        </div>
      </div>

      {/* Phase Cards */}
      <div className="grid grid-cols-3 gap-4">
        {(["design", "develop", "deploy"] as Phase[]).map((phase, index) => {
          const stats = getPhaseStats(phase);
          const colors = {
            design: "bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800",
            develop: "bg-yellow-50 dark:bg-yellow-950 border-yellow-200 dark:border-yellow-800",
            deploy: "bg-green-50 dark:bg-green-950 border-green-200 dark:border-green-800",
          };
          const iconBg = {
            design: "bg-blue-500",
            develop: "bg-yellow-500",
            deploy: "bg-green-500",
          };
          return (
            <div key={phase} className="flex items-center">
              <Card className={`flex-1 ${colors[phase]}`}>
                <CardContent className="pt-4 pb-4">
                  <div className="flex items-center gap-4">
                    <div className={`p-3 rounded-xl ${iconBg[phase]} text-white`}>
                      {getPhaseIcon(phase)}
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold capitalize">{phase}</span>
                        <span className="text-sm text-muted-foreground">{stats.total} agents</span>
                      </div>
                      <div className="flex items-center gap-4 text-sm text-muted-foreground mt-1">
                        <span className="flex items-center gap-1">
                          <span className="h-2 w-2 bg-green-500 rounded-full" />
                          {stats.active} active
                        </span>
                        <span className="flex items-center gap-1">
                          <AlertCircle className="h-3 w-3" />
                          {stats.findings} findings
                        </span>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
              {index < 2 && <ArrowRight className="h-6 w-6 mx-2 text-muted-foreground" />}
            </div>
          );
        })}
      </div>

      {/* Context Graph */}
      <Card>
        <CardHeader className="py-4">
          <div className="flex items-center justify-between">
            <CardTitle className="text-lg">Context Graph</CardTitle>
            <span className="text-sm text-muted-foreground">
              Click a node to inspect its context schema
            </span>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4">
            {(["design", "develop", "deploy"] as Phase[]).map((phase, index) => {
              const bgColors = {
                design: "bg-blue-50 dark:bg-blue-950/50",
                develop: "bg-yellow-50 dark:bg-yellow-950/50",
                deploy: "bg-green-50 dark:bg-green-950/50",
              };
              const textColors = {
                design: "text-blue-600 dark:text-blue-400",
                develop: "text-yellow-600 dark:text-yellow-400",
                deploy: "text-green-600 dark:text-green-400",
              };
              return (
                <div key={phase} className="flex items-center">
                  <div className={`flex-1 p-4 rounded-lg ${bgColors[phase]}`}>
                    <div className={`flex items-center gap-2 mb-3 ${textColors[phase]}`}>
                      {getPhaseIcon(phase)}
                      <span className="font-medium capitalize">{phase}</span>
                    </div>
                    <div className="space-y-2">
                      {contextItems[phase].map((item, i) => (
                        <div
                          key={i}
                          className="flex items-center gap-2 text-sm bg-white dark:bg-gray-900 p-2 rounded border"
                        >
                          <span
                            className={`h-2 w-2 rounded-full ${
                              item.status === "complete" ? "bg-blue-500" : "bg-yellow-500"
                            }`}
                          />
                          {item.name}
                        </div>
                      ))}
                    </div>
                  </div>
                  {index < 2 && (
                    <div className="flex flex-col items-center mx-2">
                      <ArrowRight className="h-5 w-5 text-muted-foreground" />
                      <span className="text-xs text-muted-foreground">context</span>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Agent Stack */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-xl font-semibold">Agent Stack</h2>
            <p className="text-sm text-muted-foreground">
              Six principal-level agents across the SDLC. Each badge shows the context they are
              currently holding.
            </p>
          </div>
          <Tabs value={activeFilter} onValueChange={(v) => setActiveFilter(v as "all" | Phase)}>
            <TabsList>
              <TabsTrigger value="all">All</TabsTrigger>
              <TabsTrigger value="design">Design</TabsTrigger>
              <TabsTrigger value="develop">Develop</TabsTrigger>
              <TabsTrigger value="deploy">Deploy</TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        <div className="space-y-4">
          {filteredAgents.map((agent) => (
            <Card key={agent.id} className="hover:shadow-md transition-shadow">
              <CardContent className="py-4">
                <div className="flex items-start justify-between">
                  <div className="flex items-start gap-4">
                    <div
                      className={`p-3 rounded-lg ${
                        agent.phase === "design"
                          ? "bg-blue-100 text-blue-600 dark:bg-blue-900 dark:text-blue-300"
                          : agent.phase === "develop"
                            ? "bg-yellow-100 text-yellow-600 dark:bg-yellow-900 dark:text-yellow-300"
                            : "bg-green-100 text-green-600 dark:bg-green-900 dark:text-green-300"
                      }`}
                    >
                      {agent.icon}
                    </div>
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <h3 className="font-semibold">{agent.name}</h3>
                        {getStatusBadge(agent.status)}
                        {getPhaseBadge(agent.phase)}
                      </div>
                      <p className="text-sm text-muted-foreground mb-2">{agent.shortDescription}</p>
                      <p className="text-sm text-muted-foreground mb-3">{agent.description}</p>
                      {agent.holdingContext && (
                        <div className="flex items-center gap-2 mb-3">
                          <Badge variant="outline" className="bg-blue-50 dark:bg-blue-950">
                            <span className="mr-1">📎</span>
                            Holding:{" "}
                            <span className="text-blue-600 dark:text-blue-400 ml-1">
                              {agent.holdingContext.name}
                            </span>
                          </Badge>
                        </div>
                      )}
                      <div className="flex items-center gap-2 flex-wrap">
                        {agent.capabilities.map((cap, i) => (
                          <Badge key={i} variant="outline" className="text-xs">
                            {cap}
                          </Badge>
                        ))}
                      </div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="flex items-center gap-1 text-sm text-muted-foreground mb-2">
                      <Clock className="h-4 w-4" />
                      {agent.lastActive}
                    </div>
                    <div className="text-2xl font-bold">{agent.findings}</div>
                    <div className="text-xs text-muted-foreground">findings</div>
                    <div className="flex items-center gap-2 mt-3">
                      <Button variant="outline" size="sm">
                        <Settings className="h-4 w-4 mr-1" />
                        Configure
                      </Button>
                      <Button size="sm">
                        Open <ArrowRight className="h-4 w-4 ml-1" />
                      </Button>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      {/* Placeholder Notice */}
      <Card className="border-dashed border-2 border-muted">
        <CardContent className="py-8 text-center">
          <div className="flex flex-col items-center gap-4">
            <div className="p-4 rounded-full bg-muted">
              <Settings className="h-8 w-8 text-muted-foreground" />
            </div>
            <div>
              <h3 className="font-semibold mb-1">Synthesis Integration Coming Soon</h3>
              <p className="text-sm text-muted-foreground max-w-md mx-auto">
                This is a placeholder for Kamal&apos;s Synthesis threat modeling service. The full
                integration will include STRIDE threat modeling, compliance validation, and
                automated security remediation.
              </p>
            </div>
            <Button variant="outline">View Synthesis Documentation</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
