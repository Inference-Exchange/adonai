import { Box, Text, createCliRenderer } from "@opentui/core"

import { dashboardLines, type DashboardModel } from "./dashboard"
import { initLines, type InitModel } from "./init"
import { createSupervisorClient, type AgentDefinition } from "./supervisor"
import { theme } from "./theme"

const client = createSupervisorClient(process.env.ADONAI_SUPERVISOR_URL)
const mode = process.argv.includes("--dashboard") ? "dashboard" : "init"

const starterModel = process.env.ADONAI_STARTER_MODEL ?? "llama3.2:3b"

if (process.argv.includes("--check")) {
  const model = await loadModel()
  console.log(renderLines(model).join("\n"))
  process.exit(0)
}

const renderer = await createCliRenderer({
  exitOnCtrlC: true,
  screenMode: "alternate-screen",
  targetFps: 20,
  useMouse: false,
})

async function renderDashboard(): Promise<void> {
  const model = await loadModel()
  const content = renderLines(model).join("\n")

  renderer.root.remove("dashboard")
  renderer.root.add(
    Box(
      {
        id: "dashboard",
        border: true,
        borderStyle: "rounded",
        borderColor: model.kind === "online" ? theme.surface.onlineBorder : theme.surface.offlineBorder,
        padding: 1,
        flexDirection: "column",
        width: "100%",
        height: "100%",
        title: model.kind === "online" ? (mode === "init" ? "Adonai init" : "device runtime") : "supervisor unavailable",
      },
      Text({
        content,
        fg: model.kind === "online" ? theme.text.primary : theme.text.warning,
      }),
    ),
  )
  renderer.requestRender()
}

renderer.keyInput.on("keypress", (key) => {
  if (key.name === "q") {
    renderer.destroy()
    process.exit(0)
  }

  if (key.name === "r") {
    void renderDashboard()
  }
})

await renderDashboard()
renderer.start()

async function loadModel(): Promise<DashboardModel | InitModel> {
  try {
    const [snapshot, chat, modelPlan] = await Promise.all([
      client.status(),
      client.mockChat("device runtime online"),
      client.planModel({ model: starterModel }),
    ])
    const agentRun = await client.runAgent(
      proofAgentForPlan(modelPlan),
      "Summarise the current Adonai runtime status in one sentence.",
    )
    const runs = await client.listRuns(10)

    return {
      kind: "online",
      snapshot,
      chat,
      modelPlan,
      agentRun,
      runs,
      refreshedAt: new Date(),
    }
  } catch (error) {
    return {
      kind: "offline",
      error: error instanceof Error ? error.message : String(error),
      refreshedAt: new Date(),
    }
  }
}

function renderLines(model: DashboardModel | InitModel): string[] {
  return mode === "init" ? initLines(model) : dashboardLines(model)
}

function proofAgentForPlan(modelPlan: { runnable_now: boolean; recommended_engine: string | null; model: string }): AgentDefinition {
  if (modelPlan.runnable_now && modelPlan.recommended_engine === "ollama.local") {
    return {
      id: "local-model-proof",
      name: "Local Model Proof",
      description: "Verifies Adonai can execute a real local model through the supervisor.",
      model: {
        provider: "ollama",
        name: modelPlan.model,
        temperature: 0.2,
        max_tokens: 160,
      },
      loop: {
        kind: "react",
        system_prompt: "You are a terse local runtime operator. Answer with concrete runtime facts only.",
        max_steps: 1,
      },
      triggers: [{ kind: "manual" }],
      state_dir: "~/.adonai/state/local-model-proof",
    }
  }

  return {
    id: "supervisor-smoke",
    name: "Supervisor Smoke Test",
    description: "Verifies the supervisor can execute an agent through the runtime API.",
    model: {
      provider: "mock",
      name: "test-model",
      temperature: 0.2,
    },
    loop: {
      kind: "react",
      system_prompt: "You are a terse local runtime operator.",
      max_steps: 1,
    },
    triggers: [{ kind: "manual" }],
    state_dir: "~/.adonai/state/supervisor-smoke",
  }
}
