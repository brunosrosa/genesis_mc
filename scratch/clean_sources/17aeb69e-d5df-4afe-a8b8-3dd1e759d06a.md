# Agentgateway: The AI-Native Gateway - Solo.io
Source URL: https://www.solo.io/products/agentgateway

Source Type: web_page

Source ID: 17aeb69e-d5df-4afe-a8b8-3dd1e759d06a


Built in Rust • Linux Foundation Project
The Gateway Built for AI
AI agents need gateways that understand sessions, context, bidirectional communication, and agentic protocols natively — That's why we built agentgateway.
Gemini
MCP Servers
Sales Agent
OpenAI
Anthropic
Gemini
MCP Servers
Sales Agent
OpenAI
Anthropic
How It Works
One Gateway.
Three Powerful Use Cases
Click on a use case to see how Agentgateway connects your AI infrastructure.
Trace every request and token
Real-time logging, consumption metrics, and end-to-end tracing with OpenTelemetry. Know exactly who’s using what model, when, and at what cost.
Use one API for any model
OpenAI-compatible endpoint works with any provider—OpenAI, Anthropic, Azure, Gemini, and self-hosted. Switch models without changing code.
Block prompt attacks and data leaks
Enforce inline guardrails on every prompt and response. Define custom semantic rules to stop threats before they reach your models.
Centralize keys and eliminate sprawl
Stop sharing provider keys across teams. Secure them in centralized storage with enterprise IAM integration and fine-grained access control.
Set limits and stop budget surprises
Apply request and token-based rate limits to prevent budget overruns and provider throttling. Predictable costs, no surprise bills.
Sandbox shadow MCP access
MCP servers can dynamically request backend URLs - creating shadow access. Validate, sandbox, and observe every elicited URL from a single control point.
Federate agents with A2A protocol
Connect agents across boundaries with Google’s Agent-to-Agent protocol. Discover capabilities and collaborate securely.
Know who did what and when
Complete audit trails for every tool call and agent interaction. Essential for compliance, debugging, and incident response.
Turn REST APIs into MCP tools instantly
Drop in any OpenAPI spec and instantly expose it as MCP tools. No code changes required—just configure and go.
Apply one policy to every tool
Global rate limits, quotas, and access controls from a single control plane. Consistent governance across every MCP server.
Trace every hop and kill blind spots
Full call-chain observability from agent to tool to backend. Deep metrics, logging, and tracing for every interaction.
Isolate fine-tuned models by use case
Route specific users and workloads to specialized fine-tuned models. Right request, right model, every time.
Prioritize critical workloads
Priority scheduling by use case and model. When capacity is tight, mission-critical requests never wait in line.
Slash latency and speed up TTFT
Real-time inference metrics route requests to models with available capacity. Lower latency, faster time-to- first-token.
Maximize GPU utilization
Smart routing to inference pools with llm-d integration. Isolate prefill and decode stages to extract maximum efficiency.
Route by context not just load
Context-aware routing that reads request metadata and directs traffic to the right model—including fine-tuned variants.
Insane Mode
Built in Rust to perform
and adapt as fast as AI itself
We all know this time is different. Which is why Agentgateway is purpose-built for AI. This means no technical debt. Zero-cost abstractions. No garbage collection. No locking limitations. Just high throughput and low latency at inference time.
Independent benchmarks via gateway-api-bench
300×
Leaner. Meaner. Faster.
30MB
vs
9GB
35x
Requests that actually scale.
165k QPS
vs
4.6k QPS
122x
Blink and you'll miss it.
0.09ms
vs
11ms
Weekly
Ship features, not excuses.
MCP
A2A
OAuth
Linux Foundation
No vendor lock-in. Ever.
Apache 2.0
Vendor Neutral
300×
Leaner. Meaner. Faster.
30MB
vs
9GB
35x
Requests that actually scale.
165k QPS
vs
4.6k QPS
122x
Blink and you'll miss it.
0.09ms
vs
11ms
Weekly
Ship features, not excuses.
MCP
A2A
OAuth
CNCF
No vendor lock-in. Ever.
Apache 2.0
Vendor Neutral
Why is now different?
AI and Agents aren't APIs. It's time for first principles thinking.
Retrofitting legacy proxies for MCP and A2A creates security gaps, observability blind spots, and performance bottlenecks. See how things stack up against the novel and elevated problems AI presents.
Security & Identity
Problem: Agents act on behalf of users, invoke tools dynamically, and chain across services. API-level auth wasn't built for this.
Observability
Problem: Debugging agent behavior requires visibility into tool calls, token usage, session context, and multi-step reasoning chains—not just HTTP logs.
Performance & Scale
Problem: Agents make dozens of chained calls. 50ms overhead× 20 tool calls = 1 full second of gateway tax. At scale, this breaks user experience.
Cost Optimization
Problem: Agentic workloads are expensive. Recursive agent loops, verbose completions, and uncapped token usage can spiral costs fast. Legacy gateways can't track, limit, or attribute LLM spend — agentgateway can.
See how far we are ahead of the pack
Community vs Vendor
Problem: Agentic AI is too important to be locked into a single vendor. Open governance ensures interoperability and prevents platform tax.
Seeing is Believing
Don't take our word for it. Prove it yourself.
We've made bold claims. Put them to the test. In 15 minutes, you'll see exactly why agentgateway exists.
Download & Install
One binary, zero dependencies. Running in under 2 minutes.
curl -sL https://agentgateway.dev/install.sh|bash
Take the Challenge
Try these on other gateways. We'll wait.
MCP OAuth 2. 1
— 5 lines of config
A2A Routing
— Full context preserved
Streaming + Tools
— Interleaved execution
OpenAPI MCP
— Instant bridge
Discover more
Resources to help you succeed with Istio and Ambient Mesh.