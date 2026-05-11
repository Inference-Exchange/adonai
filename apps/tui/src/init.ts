import type { AgentRunRecord, ChatCompletionResponse, ModelRunPlan, SupervisorSnapshot } from "./supervisor"

export type InitModel =
  | {
      kind: "online"
      snapshot: SupervisorSnapshot
      modelPlan: ModelRunPlan
      agentRun: AgentRunRecord
      chat: ChatCompletionResponse
      runs: AgentRunRecord[]
      refreshedAt: Date
    }
  | {
      kind: "offline"
      error: string
      refreshedAt: Date
    }

export function initLines(model: InitModel): string[] {
  if (model.kind === "offline") {
    return [
      "Adonai init",
      "",
      "Supervisor offline",
      model.error,
      "",
      "Start it with:",
      "  cargo run -p adonai-supervisor",
      "",
      "Keys: r refresh, q quit",
    ]
  }

  const { snapshot, modelPlan, agentRun, chat, runs } = model
  const hardwareSummary = summarizeHardware(snapshot)
  const localStatus = snapshot.endpoint_policy.exposure === "LoopbackOnly" ? "local only" : snapshot.endpoint_policy.exposure
  const enginePlan = modelPlan.recommended_engine ?? "none"

  return [
    "Adonai init",
    "Agent OS for owned compute",
    "",
    `1. Machine: ${hardwareSummary}`,
    `2. Privacy: ${localStatus} at ${snapshot.endpoint_policy.bind.host}:${snapshot.endpoint_policy.bind.port}`,
    `3. Engine plan: ${enginePlan} for ${modelPlan.model}`,
    `4. Runnable now: ${modelPlan.runnable_now ? "yes" : "no"}`,
    "",
    "Why Adonai chose this",
    ...prefix(modelPlan.reasons.slice(0, 3), "- "),
    ...prefix(modelPlan.missing.slice(0, 3), "- Missing: "),
    ...prefix(modelPlan.warnings.slice(0, 3), "- Warning: "),
    "",
    "Proof run",
    `${agentRun.id}: ${agentRun.status}`,
    `Provider: ${agentRun.provider ?? "pending"}/${agentRun.model ?? "pending"}`,
    agentRun.final_message?.content ?? agentRun.error ?? "No final message yet",
    "",
    "Control API smoke",
    `${chat.provider}/${chat.model}: ${chat.message.content}`,
    "",
    "Recent runs",
    ...formatRuns(runs),
    "",
    `Refreshed: ${model.refreshedAt.toLocaleTimeString()}`,
    "Keys: r refresh, q quit",
  ]
}

function summarizeHardware(snapshot: SupervisorSnapshot): string {
  const platform = snapshot.hardware.platform
  const memoryGb = bytesToGb(snapshot.hardware.memory.total_bytes)
  const accelerator = snapshot.hardware.accelerators[0]?.name ?? "no accelerator detected"
  return `${platform.os} ${platform.architecture}, ${snapshot.hardware.cpu_brand}, ${memoryGb} GB, ${accelerator}`
}

function formatRuns(runs: AgentRunRecord[]): string[] {
  if (runs.length === 0) {
    return ["No runs persisted yet"]
  }

  return runs
    .slice(0, 5)
    .map((run) => `${run.id}  ${run.status}  ${run.agent_id}  ${new Date(run.created_at_ms).toLocaleTimeString()}`)
}

function prefix(items: string[], prefixText: string): string[] {
  return items.map((item) => `${prefixText}${item}`)
}

function bytesToGb(bytes: number): string {
  return (bytes / 1024 / 1024 / 1024).toFixed(1)
}
