# KeefeOS AI agent security model

**Status:** Design specification only. KeefeOS does not yet have the user-space isolation, system-call interface, durable credential store, complete networking stack, or policy broker required to implement this securely.

## 1. Product goal

The long-term product goal is an operating system with personal AI agents that can help users complete tasks across computers, tablets, and phones. Agents may use a hosted LLM API, a local model, MCP servers, or ordinary APIs when that platform supports them.

The design must remain provider-neutral and offline-capable where practical. A provider/API key is an optional user-controlled integration, not a boot requirement and not a kernel secret.

## 2. Trust boundaries

Treat these components as separate trust domains:

1. **Kernel:** enforces process memory, privilege, IPC, and resource boundaries. It does not prompt an LLM, connect to an MCP server, or execute model-generated shell text.
2. **Agent service (future user space):** manages conversations and plans. It has no direct authority to bypass the policy broker.
3. **Policy broker (future user space, backed by kernel-enforced capabilities):** authenticates requests, checks grants, scopes files/devices/network destinations, applies rate and resource limits, records decisions, and asks for approval where required.
4. **Tool adapters:** MCP clients, local tools, and provider APIs. Tool results, server descriptions, web pages, files, and model output are untrusted input.
5. **LLM provider or local model:** generates suggestions and proposed tool calls. It is not an authority source and cannot grant itself permissions.
6. **User interface:** shows requested actions, targets, data sharing, risks, and result logs; exposes cancellation and revocation.

The actual authorization boundary must be enforced by the OS. Prompt wording alone is not a security control.

## 3. Permission policy

Use least privilege and explicit, revocable grants.

| Action class | Initial policy for a future MVP |
|---|---|
| Read non-sensitive OS status and display help | Allowed only through typed, read-only tools |
| Browse a user-selected directory | Require a scoped directory grant; deny path traversal and access outside the grant |
| Draft or edit a file | Use a proposed diff and require user review before applying |
| Execute a command, install software, change system settings | Require a visible preview and explicit confirmation; use a sandbox and restricted account |
| Send messages, upload data, access an external API, or expose private files | Explain the destination and data being sent; require confirmation and narrow grants |
| Delete data, alter boot/security settings, change credentials, or perform irreversible operations | Deny by default in the first version; later require strong confirmation and recoverability |
| Kernel memory, raw disk, secrets, privilege escalation, or arbitrary root shell | Never expose as a general-purpose agent tool |

Every grant should be scoped by user/session, resource, operation, expiry or revocation state, and provenance. If the current kernel cannot enforce a grant, do not pretend the grant is a security boundary.

## 4. MCP and API design

- Use a typed tool registry; do not turn arbitrary JSON into shell commands.
- Validate JSON-RPC envelopes, method names, argument schemas, lengths, timeouts, response sizes, and error cases.
- Use an explicit allowlist of tools and server endpoints; apply network policy independent of model output.
- Treat MCP server tool descriptions and results as untrusted data. They may contain prompt-injection attempts or ask the agent to reveal secrets.
- Do not let a remote server silently register privileged local tools.
- Require per-server grants, clear server identity, cancellation, bounded concurrency, and audit entries.
- Keep authentication secrets out of prompts, logs, crash reports, and command history.
- Do not send secrets or private files to a provider unless the user has approved that data flow.

## 5. API-key handling

A safe implementation depends on the credential facilities that do not exist yet:

- Never compile a user's provider key into the kernel or a public build.
- Never commit keys into Git or expose them in shell arguments, logs, telemetry, or screenshots.
- Prefer an OS-managed encrypted credential service with keys inaccessible to the agent except through scoped operations. This requires a real key-management and threat model first.
- Before that service exists, provider integration should remain a development-only opt-in configuration and must not be described as secure OS credential storage.
- Support key revocation, provider-specific scopes, usage/rate limits, and clear disclosure of outbound data.

## 6. Offline and local-model behavior

The OS must still boot and support core functions without Internet, API keys, a model, or an MCP server. Local model inference is a user-space workload subject to memory, CPU/GPU, battery, privacy, and resource limits. The kernel should not assume that all target devices have enough compute to run an LLM locally.

Hosted and local providers should implement the same documented provider interface, but must be isolated behind adapters and must not bypass policy checks.

## 7. Reliability and safety properties

Before an agent executes a tool, it should have:
- a validated schema and a bounded execution deadline;
- a known principal and explicit permission;
- a risk classification and confirmation policy;
- a trace ID, audit event, and cancellation path;
- resource/time/output limits;
- safe handling for errors, partial results, retries, and duplicated requests.

Agent actions should support dry-run/preview and undo where technically possible. A successful tool invocation is not proof that a task's desired outcome occurred; verify the actual postcondition before reporting completion.

## 8. Implementation gates

Do not ship an agent capable of executing system commands until these prerequisites are complete and tested:

1. Real process isolation and privilege transitions.
2. A stable syscall and IPC contract with per-operation authorization.
3. A broker that cannot grant more privilege than the kernel enforces.
4. Filesystem, network, credential, process and service boundaries with tests.
5. A sandboxed command runner with resource controls and cancellation.
6. An audit log that identifies the request, principal, tool, target, approval, and result without recording secrets.
7. Threat-model review and adversarial tests for prompt injection, malicious tool responses, confused-deputy bugs, path traversal, and privilege escalation.

At present these are design requirements, not implemented product features.

## References and engineering basis

These external documents inform the requirements above; linking them does not imply certification or compliance:

- [OWASP MCP Top 10](https://owasp.org/projects/mcp-top-10) — token/secret exposure, scope creep, tool poisoning, command execution, authentication/authorization, and auditability risks for MCP integrations.
- [OWASP Top 10 for Agentic Applications](https://genai.owasp.org/2025/12/09/owasp-top-10-for-agentic-applications-the-benchmark-for-agentic-security-in-the-age-of-autonomous-ai/) — agent goal hijacking, tool misuse, identity/privilege abuse, supply-chain risk, unexpected code execution, and memory/context poisoning.
- [NIST AI RMF: Generative AI Profile (NIST AI 600-1)](https://doi.org/10.6028/NIST.AI.600-1) — lifecycle risk assessment, governance, measurement, and ongoing management for generative AI systems.
- [The Rustonomicon: How Safe and Unsafe Interact](https://doc.rust-lang.org/stable/nomicon/safe-unsafe-meaning.html) — safe APIs must uphold Rust's safety guarantees even when called by code that misuses ordinary safe inputs.
