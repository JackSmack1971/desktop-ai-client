import { emitHookError, emitJson } from "./helpers/io.js";

try {
  emitJson({
    hookSpecificOutput: {
      hookEventName: "SessionStart",
      sessionTitle: "desktop-ai-client",
      additionalContext: [
        "Read order:",
        "AGENTS.md, docs/README.md, docs/agent-context.md, docs/architecture.md, docs/privacy-boundaries.md,",
        "docs/provider-routing.md, docs/threat-model.md, docs/command-inventory.md, .planning/PROJECT.md,",
        ".planning/REQUIREMENTS.md, .planning/ROADMAP.md.",
        "High-risk surfaces: secrets, file-intake, provider-routing, storage-migrations, telemetry, release-evidence."
      ].join(" ")
    }
  });
} catch (error) {
  emitHookError("SessionStart", error);
  console.error(error.stack || error.message);
  process.exit(2);
}
