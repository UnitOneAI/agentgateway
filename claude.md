# Agentgateway (UnitOne Fork) - Initial Intent

## Purpose
UnitOne's fork of the open source agentgateway project. Agentgateway is a data plane optimized for agentic AI connectivity, providing drop-in security, observability, and governance for agent-to-agent (A2A) and agent-to-tool communication, supporting interoperable protocols including Agent2Agent (A2A) and Model Context Protocol (MCP).

**Repository Structure:**
- **Upstream (Official)**: `https://github.com/agentgateway/agentgateway`
- **UnitOne Fork**: `git@github.com:UnitOneAI/agentgateway.git`
- **Purpose of Fork**: UnitOne-specific features, testing, and potential contributions back to upstream

## Project Vision & Strategic Direction

### Long-term Goals (Q1-Q2 2026)
**Vision**: Become the standard security and observability layer for all MCP (Model Context Protocol) communications in UnitOne's AI agent ecosystem.

**Key Objectives**:
- Production-ready MCP security guard framework with pluggable detection probes
- Real-time threat detection and prevention for agent-to-tool communication
- Observable and governable multi-agent workflows
- Contribute core security features back to upstream agentgateway project

### Current Phase: **Security Guard MVP** (Weeks 1-4, Q1 2026)
**Goal**: Demonstrate MCP security monitoring with 3-4 detection probes
**Success Criteria**: Live demo showing threat prevention on Azure deployment
**Status**: Tool Poisoning and PII Detection probes complete and demo-ready

## Recent Significant Changes

### Architecture: MCP Security Guards Framework (Jan 2026)
**What Changed**: Added native security guard system for MCP protocol monitoring
- **Location**: `crates/agentgateway/src/mcp/security/native/`
- **Capabilities**: Real-time request/response inspection, threat detection, policy enforcement
- **Probes Implemented**:
  - Tool Poisoning Detection (`tool_poisoning.rs`) - detects prompt injection, system override attempts
  - PII Detection (`pii.rs`) - masks/blocks sensitive information in transit
- **Current Limitation**: Guard configuration baked into Docker image

**Why This Matters**: MCP protocol enables powerful tool access for LLMs, creating new attack surfaces:
- Malicious tools can inject prompts to manipulate LLM behavior
- Tools may leak PII or sensitive data through responses
- Need runtime protection layer without modifying client/server implementations

**Technical Foundation**: Rust-based guards with regex pattern matching, JSON schema validation, and configurable actions (allow/deny/mask).

### Integration: UnitOne Control Plane (In Design, Jan 2026)
**What's Coming**: AgentGateway Dashboard will integrate with UnitOne's broader control plane
- **Question**: Where's the boundary between MCP-specific features vs. wider agent observability?
- **UI Strategy**: Embedded iframe vs. standalone dashboard with shared identity system
- **Impact**: Unified agent governance across MCP and non-MCP communication patterns

## Current Focus Areas (Jan 12-19, 2026)

### Theme 1: Runtime Security Guard Configuration
**Goal**: Move guard configuration from baked-in Docker image to runtime dashboard API
**Why Now**: Current approach requires image rebuild for every config change; not scalable for production
**Impact**:
- Enable dynamic security policies per MCP server
- Faster iteration on detection rules without redeployment
- Support customer-specific security requirements

**Technical Approach**: Dashboard CRUD APIs for guard configuration, persistent storage, runtime policy loading

### Theme 2: Stateful Threat Detection
**Goal**: Implement "rug pull" detection - tools that advertise different capabilities than they deliver
**Why Now**: Current probes are stateless (per-request); need to validate framework supports cross-request pattern matching
**Impact**:
- Proves guard framework can handle sophisticated multi-step attacks
- Enables detection of behavioral anomalies over time
- Opens path for ML-based anomaly detection

**Technical Challenge**: Requires memory/context tracking across requests while maintaining performance

### Theme 3: Production Validation
**Goal**: End-to-end testing, stress testing, upstream compatibility
**Why Now**: Moving from proof-of-concept to production Azure deployment
**Impact**:
- Confidence in reliability and performance under load
- Ensures clean merge path to upstream for potential contributions
- Validates guard overhead is acceptable (<50ms P99 latency target)

## Evolution Notes

### Fork Strategy (Jan 2026)
**Divergence Point**: UnitOne fork specialized in MCP security while upstream focuses on general-purpose agent gateway
- **Upstream**: Protocol-agnostic agent routing and observability
- **UnitOne**: Deep MCP security specialization with detection probes
- **Contribution Path**: Keep security guard framework upstreamable; contribute when proven in production

### Build & Deployment (Jan 2026)
**Shift**: From manual builds to dual-mode automation (covered in unitone-agentgateway repo)
- Production: ACR Task auto-builds on push to main
- Development: Local builds for rapid iteration
- Infrastructure managed in sibling terraform repository

## Core Functionality
- **Protocol Support**: Native A2A and MCP protocol implementation
- **Security**: RBAC system for agent and tool access control
- **Observability**: Built-in logging, metrics, and tracing for agent interactions
- **Performance**: Rust-based for high throughput and low latency
- **Interoperability**: Bridge between different agent frameworks and environments
- **Developer Experience**: CLI tools, SDKs, and comprehensive API

## Key Components Protected

### Core Rust Application (`src/`)
- **Protocol Handlers**: A2A and MCP protocol implementation
- **RBAC Engine**: Role-based access control for agents and tools
- **Proxy Layer**: Request routing and transformation
- **Auth System**: Authentication and authorization middleware
- **Observability Stack**: Metrics, logs, and traces collection

### Web UI (`ui/`)
- **Next.js Application**: Management console
- **Agent Registry**: Visual agent and tool management
- **Security Dashboard**: RBAC policy configuration
- **Observability UI**: Real-time metrics and logs

### Protocol Implementations
- **A2A Protocol**: Google's Agent2Agent standard
- **MCP Protocol**: Anthropic's Model Context Protocol
- **HTTP/gRPC**: Transport layer support
- **WebSocket**: Real-time bidirectional communication

## Critical Workflows

1. **Agent-to-Agent Communication**
   ```
   Agent A → Agentgateway → Auth Check → RBAC Validation
   → Protocol Translation → Agent B → Response → Logging
   ```

2. **Tool Invocation**
   ```
   Agent → Agentgateway → Tool Discovery → Permission Check
   → Tool Execution → Result Return → Audit Log
   ```

3. **Policy Enforcement**
   ```
   Request → Identity Extraction → Policy Lookup → Decision
   → Allow/Deny → Log Decision → Execute/Reject
   ```

4. **Observability Pipeline**
   ```
   Event → Metrics Collection → Log Aggregation → Trace Correlation
   → Export to Backend → Dashboard Update
   ```

## Security Boundaries

### Authentication
- OAuth 2.0 / OIDC integration
- API key management
- mTLS for service-to-service
- JWT token validation

### Authorization
- RBAC policies for agents and tools
- Fine-grained permission model
- Policy as code (declarative YAML/JSON)
- Dynamic policy updates

### Data Protection
- TLS for all external connections
- Request/response encryption
- PII redaction in logs
- Secrets management integration

### Network Isolation
- Support for service mesh integration
- VPC/VNET deployment
- Firewall rule compatibility
- Rate limiting and DDoS protection

## Invariants to Protect

1. **Protocol Compliance**: A2A and MCP specifications must be followed
2. **RBAC Model**: Policy structure and evaluation logic
3. **API Contracts**: REST and gRPC API stability
4. **CLI Interface**: Command structure and core flags
5. **Configuration Schema**: YAML/JSON config file format
6. **Database Schema**: Policy and metadata storage

## Dependencies
- **Rust**: >= 1.70 (core language)
- **Tokio**: Async runtime
- **Axum/Tonic**: HTTP/gRPC frameworks
- **Next.js**: UI framework
- **PostgreSQL/Redis**: Optional persistence layer

## What Should Not Change Without Review

### Breaking Changes
- A2A/MCP protocol wire format
- RBAC policy evaluation logic
- Core API endpoints (versioning required)
- CLI command structure
- Configuration file schema
- Database migrations

### Critical Paths
- Authentication middleware
- Authorization checks
- Protocol serialization/deserialization
- Error handling and propagation
- Observability data collection

## Approved Extension Points
- New protocol support (beyond A2A/MCP)
- Additional auth providers
- Custom RBAC policy functions
- Observability backend integrations
- UI dashboards and visualizations
- Performance optimizations
- New deployment targets (k8s operators, etc.)

## Interoperability Standards

### Supported Protocols
- **A2A (Agent2Agent)**: Google's agent interoperability standard
- **MCP (Model Context Protocol)**: Anthropic's tool/context protocol
- **HTTP REST**: Standard HTTP/JSON APIs
- **gRPC**: High-performance RPC

### Framework Compatibility
- LangChain / LangGraph
- AWS AgentCore / Bedrock
- Anthropic Claude
- OpenAI Assistants
- Custom agent frameworks

## Performance Targets
- **Latency**: P99 < 50ms for local routing
- **Throughput**: 10K+ requests/second per instance
- **Resource Usage**: < 100MB memory baseline
- **Scalability**: Horizontal scaling to 100+ instances

## Community & Ecosystem
- **License**: Apache 2.0
- **Upstream GitHub**: github.com/agentgateway/agentgateway
- **UnitOne Fork**: github.com/UnitOneAI/agentgateway
- **Discord**: Active community support
- **Documentation**: Comprehensive guides and API docs
- **Integration with kgateway**: Kubernetes Gateway API support

## Fork Management

### Sync Strategy
- Regularly pull updates from upstream
- Test UnitOne-specific features against upstream changes
- Contribute bug fixes and features back to upstream when appropriate
- Maintain clean commit history for potential upstreaming

### UnitOne-Specific Changes
- Document all fork-specific modifications
- Keep customizations minimal to ease upstream merging
- Prefer configuration over code changes where possible
- Tag releases with UnitOne versioning scheme

---
**Document Version**: 1.0
**Created**: 2026-01-08
**Last Updated**: 2026-01-08
**Status**: Fork of Open Source Project (Active Development)
**Upstream**: https://github.com/agentgateway/agentgateway
