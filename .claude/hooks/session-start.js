import { emitJson } from "./helpers/io.js";

emitJson({
  status: "ok",
  hook: "session-start",
  readOrder: [
    "AGENTS.md",
    "docs/README.md",
    "docs/agent-context.md",
    "docs/architecture.md",
    "docs/privacy-boundaries.md",
    "docs/provider-routing.md",
    "docs/threat-model.md",
    "docs/command-inventory.md",
    ".planning/PROJECT.md",
    ".planning/REQUIREMENTS.md",
    ".planning/ROADMAP.md"
  ],
  highRiskSurfaces: [
    "secrets",
    "file-intake",
    "provider-routing",
    "storage-migrations",
    "telemetry",
    "release-evidence"
  ]
});

