# Agentgateway Review: A Feature-Rich New AI Gateway - DEV Community
Source URL: https://dev.to/spacewander/agentgateway-review-a-feature-rich-new-ai-gateway-53lm

Source Type: web_page

Source ID: df3b4efd-5a71-46c9-96ab-c83d5c37baba


Introduction
agentgateway is a data plane developed by solo specifically for AI scenarios. The data plane is written in Rust and can be configured via xDS (a gRPC-based protocol) and YAML. Recently they decided to replace kgateway’s AI data plane from Envoy to agentgateway. I expect the enterprise version of Gloo will follow. Previously, most AI-related data-plane features were implemented in Envoy calling a Go sidecar via ext_proc, and I guess the real-world results were mediocre.
This gateway supports four AI scenarios:
- MCP
- A2A
- Proxying inference requests to LLM providers
- Load balancing for inference services
Below I explain each of these scenarios. Note I’m discussing the open-source agentgateway — some features may exist only in the enterprise edition and are outside the scope of this doc.
MCP
agentgateway was originally started to address the difficulty of handling stateful MCP requests in existing Envoy data planes. So its MCP support is the most complete.
By default, agentgateway treats MCP as a stateful protocol. It has a SessionManager struct responsible for session creation and maintenance (code link). But this SessionManager is a local in-process store, which means if you run multiple agentgateway instances there’s no guarantee a client will hit the same SessionManager each time. If you want sticky sessions toward upstreams, it’s actually simpler to consistent-hash on the MCP-Session-ID header so the same session ID routes to the same backend even if requests land on different agentgateway instances. Extending SessionManager to use a remote store is another solution, but it’s more expensive. To me, making MCP stateful by default is a mistake. I’m glad they plan to make MCP a default stateless protocol.
When there is more than one backend, agentgateway enables MCP multiplexing. For example, with tools: when listing tools, agentgateway sends tools/list to every backend, then rewrites tool names to the format ${backend_name}_${tool_name}
. When a tool call comes in, agentgateway routes to the actual backend. For methods that can’t be multiplexed, it returns an invalid method error.
Besides forwarding to MCP backends, agentgateway supports converting RESTful APIs to MCP tools using an OpenAPI spec. Impressively, it supports using an entire spec as a backend and includes a fair amount of schema-parsing code. agentgateway positions itself here as an MCP-to-RESTful-API forwarder; it does not itself manage the RESTful APIs described in the OpenAPI spec. Some details are still missing — for example, bodies only support application/json, HTTPS upstreams aren’t supported yet, structured output is not yet supported, etc. There are also finer points (e.g., handling of additionalProperties) I haven’t dug fully into.
agentgateway implements OAuth-based MCP authentication. It exposes protected resource metadata at paths like /.well-known/oauth-protected-resource/${resource}
. However, if one host contains multiple resources, should each resource’s route-match config explicitly include that resource’s well-known path? Otherwise you can’t guarantee the request will route to the well-known path handler. One nice thing: agentgateway adds CORS headers to metadata responses, so when an MCP client runs in a browser (e.g., the MCP inspector) you don’t need to add a separate CORS middleware.
agentgateway fetches public keys from a JWKS path to verify tokens were issued by the corresponding authorization server. There are two JWKS sources:
- The user supplies a URL or a file path.
- The JWKS URL is derived from the issuer URL and issuer type.
The code that gets public keys from JWKS appears to be called only when parsing configuration. So the JWKS does not seem to be periodically refreshed.
Authorization is also implemented via OAuth. It uses a list of CEL expressions as filters, matching on JWT fields and MCP attributes. Example:
mcpAuthorization:
rules:
# Allow anyone to call 'echo'
- 'mcp.tool.name == "echo"'
# Only the test-user can call 'add'
- 'jwt.sub == "test-user" && mcp.tool.name == "add"'
# Any authenticated user with the claim `nested.key == value` can access 'printEnv'
- 'mcp.tool.name == "printEnv" && jwt.nested.key == "value"'
Note: in multiplexing scenarios, mcpAuthorization runs before the tool lists are merged, so the tool names here do not include the backend-name prefix.
agentgateway provides surprisingly few MCP-related metrics — basically just an mcp_requests counter — so you can’t see details like which tools are taking the most time.
A2A
For A2A protocol scenarios, agentgateway implements two main features:
- Rewrites agent card URLs so they point to the gateway instead of the proxied backend.
- Parses A2A JSON requests and records the request method for observability.
Proxying inference requests to LLM providers
Like other AI gateways, agentgateway can proxy inference requests to LLM providers. This proxying is not just raw forwarding: it adds value such as token-based observability and rate-limiting.
When proxying SSE traffic it collects token usage and TTFT metrics. For non-SSE streaming formats (e.g., Bedrock’s AWS event stream) it provides dedicated parsers.
I’ll dive into rate limiting, prompt protection, and related features in a follow-up.
Another common capability is to lift some LLM client features into the gateway to reduce integration work — for example, smoothing differences between providers and offering an OpenAI-compatible external API.
agentgateway supports this to an extent. Its design is not a full generic "X provider to Y provider" converter; instead it implements conversions for specific routing types. Currently it supports two route types:
- OpenAI’s /v1/chat/completions
- Anthropic’s /v1/messages
In practice both /v1/chat/completions and /v1/messages are chat-style routes: OpenAI’s /v1/chat/completions is functionally equivalent to Anthropic’s /v1/messages. They implemented both separately for quick business onboarding: many code agents only implement Anthropic’s /v1/messages, and special-casing that endpoint makes it easy to immediately accept such clients. Implementing a full Anthropic-to-any-provider converter would be a much larger effort.
This area is currently roughly sufficient but incomplete. Putting aside support for embeddings, batching, etc., agentgateway does not fully support /v1/chat/completions yet — for example, structured output is not supported at the moment.
Inference Extension Support
When the gateway API inference extension (https://gateway-api-inference-extension.sigs.k8s.io/) first appeared I was skeptical. Distributed inference is a systems engineering problem; it feels presumptuous for a single scheduler implementation to try to become the standard. But with Red Hat driving the LLMD project and treating the inference extension as part of an out-of-the-box experience, the inference extension may gain traction. Red Hat has invested heavily in AI projects (e.g., vLLM) and has the resources to advance this work.
Supporting the inference extension is actually not hard. The gateway needs to forward inference requests to a scheduler (called EPP in the inference extension) via Envoy’s gRPC ext_proc protocol. The scheduler’s response includes an x-gateway-destination-endpoint header that contains the target upstream address. The gateway then forwards the inference request to that address. Practically speaking the gateway is only doing forwarding here; the core logic lives in the scheduler. I’ve wondered: if the entire request is sent to the scheduler, why not let the scheduler process the request directly instead of having the gateway forward it? Is the scheduler only capable of handling input tokens and not output tokens?
What’s the value of a self-hosted inference system? I think it’s to, under data-security constraints, be reasonably cost-competitive with external LLM providers. Large-model inference benefits from scale economics greatly — a self-hosted system is unlikely to beat cloud providers purely on price. To be more cost-effective you need scheduling innovations (e.g., better load balancing, more flexible disaggregated serving). If inference-extension support just means forwarding requests to the official scheduler, then the gateway isn’t adding meaningful value in that part of the chain.
Summary
In summary, agentgateway is impressive for a project that’s been developed for only about half a year. Its feature richness stands out. It shows a clear focus on AI scenarios, and its ambition to rebuild the data plane in Rust (to replace the prior Envoy + Go external process approach) demonstrates strong intent and potential to address AI-specific protocols and performance needs.
However, the documentation is incomplete: some implemented features (e.g., Anthropic /v1/messages support) aren’t documented, while some documented items don’t exist in the code (e.g., the MCP metric list_calls_total referenced in the docs: https://github.com/agentgateway/website/blob/02e25020b185ed34c66704d6274708a24ffe098d/content/docs/mcp/mcp-observability.md?plain=1#L18). Overall these are typical, understandable issues for a rapidly iterating early-stage open-source project and do not substantially detract from the project’s promise.
Top comments (0)