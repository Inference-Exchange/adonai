import type { AgentRunRecord, ChatCompletionResponse, ModelRunPlan, SupervisorSnapshot } from "./supervisor"

export type DashboardModel =
  | {
      kind: "online"
      snapshot: SupervisorSnapshot
      chat: ChatCompletionResponse
      modelPlan: ModelRunPlan
      agentRun: AgentRunRecord
      runs: AgentRunRecord[]
      refreshedAt: Date
    }
  | {
      kind: "offline"
      error: string
      refreshedAt: Date
    }

export function dashboardLines(model: DashboardModel): string[] {
  if (model.kind === "offline") {
    return [
      "Supervisor offline",
      "",
      model.error,
      "",
      "Start it with:",
      "  cargo run -p adonai-supervisor",
      "",
      "Keys: r refresh, q quit",
    ]
  }

  const { snapshot, chat, modelPlan, agentRun, runs } = model
  const platform = snapshot.hardware.platform
  const memoryGb = bytesToGb(snapshot.hardware.memory.total_bytes)
  const storage = snapshot.hardware.storage
    .slice(0, 3)
    .map(
      (disk) =>
        `${disk.mount_point}: ${bytesToGb(disk.available_bytes)} GB free / ${bytesToGb(disk.total_bytes)} GB`,
    )

  return [
    `${snapshot.product} supervisor`,
    `State: ${snapshot.state}    Version: ${snapshot.version}`,
    `Endpoint: ${snapshot.endpoint_policy.bind.host}:${snapshot.endpoint_policy.bind.port} (${snapshot.endpoint_policy.exposure})`,
    "",
    "Hardware",
    `Host: ${platform.hostname ?? "unknown"}`,
    `OS: ${platform.os} ${platform.os_version ?? ""} (${platform.architecture})`,
    `CPU: ${snapshot.hardware.cpu_brand}`,
    `Memory: ${memoryGb} GB total`,
    `Accelerators: ${snapshot.hardware.accelerators.map((accelerator) => accelerator.name).join(", ")}`,
    "",
    "Storage",
    ...storage,
    "",
    "Engines",
    ...snapshot.engines.engines.map(
      (engine) =>
        `${engine.adapter_id}: ${engine.health} (${engine.provenance.binary_path ?? "not found"})${formatInstalledModels(engine.installed_models)}`,
    ),
    "",
    "Engine recommendations",
    ...snapshot.engines.recommendations.map(
      (recommendation) =>
        `${recommendation.adapter_id}: ${recommendation.level} - ${recommendation.reason}`,
    ),
    "",
    "Model plan",
    `${modelPlan.model}: ${modelPlan.runnable_now ? "runnable now" : "not runnable yet"}`,
    `Engine: ${modelPlan.recommended_engine ?? "none"}    Artifact: ${modelPlan.artifact}    Memory: ${modelPlan.memory_class}`,
    ...modelPlan.reasons.slice(0, 2),
    ...modelPlan.missing.slice(0, 2).map((item) => `Missing: ${item}`),
    ...modelPlan.warnings.slice(0, 2).map((item) => `Warning: ${item}`),
    "",
    "Chat provider smoke",
    `${chat.provider}/${chat.model}: ${chat.message.content}`,
    "",
    "Agent run smoke",
    `${agentRun.id}: ${agentRun.status}`,
    `Provider: ${agentRun.provider ?? "pending"}/${agentRun.model ?? "pending"}`,
    agentRun.final_message?.content ?? agentRun.error ?? "No final message yet",
    "",
    "Recent runs",
    ...formatRuns(runs),
    "",
    `Refreshed: ${model.refreshedAt.toLocaleTimeString()}`,
    "Keys: r refresh, q quit",
  ]
}

function formatRuns(runs: AgentRunRecord[]): string[] {
  if (runs.length === 0) {
    return ["No runs persisted yet"]
  }

  return runs
    .slice(0, 5)
    .map((run) => `${run.id}  ${run.status}  ${run.agent_id}  ${new Date(run.created_at_ms).toLocaleTimeString()}`)
}

function bytesToGb(bytes: number): string {
  return (bytes / 1024 / 1024 / 1024).toFixed(1)
}

function formatInstalledModels(models: Array<{ name: string }>): string {
  if (models.length === 0) {
    return ""
  }

  return ` - models: ${models
    .slice(0, 3)
    .map((model) => model.name)
    .join(", ")}`
}
