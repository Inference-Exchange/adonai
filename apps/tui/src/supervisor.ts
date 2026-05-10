export type SupervisorHealth = {
  product: string
  status: string
}

export type SupervisorSnapshot = {
  product: string
  state: string
  version: string
  endpoint_policy: {
    bind: {
      host: string
      port: number
    }
    exposure: string
  }
  hardware: HardwareProfile
  engines: EngineProbe
}

export type HardwareProfile = {
  platform: {
    os: string
    os_version: string | null
    kernel_version: string | null
    architecture: string
    hostname: string | null
  }
  cpu_brand: string
  physical_core_count: number | null
  memory: {
    total_bytes: number
    available_bytes: number | null
  }
  storage: Array<{
    name: string
    mount_point: string
    total_bytes: number
    available_bytes: number
  }>
  accelerators: Array<{
    kind: string
    name: string
    evidence: string
  }>
  network: {
    default_exposure: string
  }
}

export type EngineProbe = {
  engines: Array<{
    adapter_id: string
    kind: string
    health: string
    capabilities: Array<{
      name: string
      supported: boolean
    }>
    provenance: {
      binary_path: string | null
      version: string | null
      source: string
    }
  }>
  recommendations: Array<{
    adapter_id: string
    level: "Preferred" | "Viable" | "Unsupported"
    reason: string
  }>
}

export type ChatCompletionResponse = {
  provider: string
  model: string
  message: {
    role: string
    content: string
  }
}

export type AgentDefinition = {
  id: string
  name: string
  description?: string
  model: {
    provider: string
    name: string
    max_tokens?: number
    temperature?: number
  }
  loop: {
    kind: "react" | "graph" | "custom"
    system_prompt?: string
    max_steps?: number
    spec_path?: string
  }
  tools?: Array<{
    name: string
    kind: "http-fetch" | "mcp" | "builtin"
    config?: Record<string, unknown>
  }>
  triggers?: Array<{
    kind: "manual" | "cron" | "webhook" | "file-watch"
    cron?: string
    path?: string
  }>
  state_dir: string
  secrets?: Array<{
    name: string
    keychain_key: string
  }>
  resources?: {
    cpu_pct?: number
    ram_mb?: number
    gpu?: "shared" | "exclusive"
  }
  lifecycle?: {
    on_start?: string
    on_stop?: string
    on_crash?: "restart" | "halt" | { backoff: { strategy: "exponential" | "linear"; max_attempts: number } }
  }
}

export type ModelPlanRequest = {
  model: string
  source?: "ollama" | "hugging-face" | "local-file"
  artifact?: "ollama-model" | "safetensors" | "gguf" | "unknown"
}

export type ModelRunPlan = {
  model: string
  source: string
  artifact: string
  recommended_engine: string | null
  runnable_now: boolean
  memory_class: string
  reasons: string[]
  missing: string[]
  warnings: string[]
}

export type RunStatus = "running" | "succeeded" | "failed"

export type AgentRunRecord = {
  id: string
  agent_id: string
  agent_name: string
  goal: string
  status: RunStatus
  provider: string | null
  model: string | null
  final_message: {
    role: string
    content: string
  } | null
  error: string | null
  created_at_ms: number
  updated_at_ms: number
}

export type SupervisorClient = {
  baseUrl: string
  health: () => Promise<SupervisorHealth>
  status: () => Promise<SupervisorSnapshot>
  engines: () => Promise<EngineProbe>
  mockChat: (content: string) => Promise<ChatCompletionResponse>
  planModel: (request: ModelPlanRequest) => Promise<ModelRunPlan>
  runAgent: (agent: AgentDefinition, goal: string) => Promise<AgentRunRecord>
  listRuns: (limit?: number) => Promise<AgentRunRecord[]>
  getRun: (runId: string) => Promise<AgentRunRecord>
}

export function createSupervisorClient(baseUrl = "http://127.0.0.1:49231"): SupervisorClient {
  return {
    baseUrl,
    health: () => getJson<SupervisorHealth>(`${baseUrl}/health`),
    status: () => getJson<SupervisorSnapshot>(`${baseUrl}/v1/status`),
    engines: () => getJson<EngineProbe>(`${baseUrl}/v1/engines`),
    mockChat: (content: string) =>
      postJson<ChatCompletionResponse>(`${baseUrl}/v1/chat/completions`, {
        provider: "mock",
        model: "test-model",
        messages: [{ role: "user", content }],
      }),
    planModel: (request: ModelPlanRequest) => postJson<ModelRunPlan>(`${baseUrl}/v1/models/plan`, request),
    runAgent: (agent: AgentDefinition, goal: string) =>
      postJson<AgentRunRecord>(`${baseUrl}/v1/agents/runs`, {
        agent,
        goal,
      }),
    listRuns: (limit = 10) => getJson<AgentRunRecord[]>(`${baseUrl}/v1/agents/runs?limit=${limit}`),
    getRun: (runId: string) => getJson<AgentRunRecord>(`${baseUrl}/v1/agents/runs/${runId}`),
  }
}

async function getJson<T>(url: string): Promise<T> {
  const response = await fetch(url)
  return parseJsonResponse<T>(response)
}

async function postJson<T>(url: string, body: unknown): Promise<T> {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  })
  return parseJsonResponse<T>(response)
}

async function parseJsonResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.text()
    throw new Error(`${response.status} ${response.statusText}: ${body}`)
  }

  return (await response.json()) as T
}
