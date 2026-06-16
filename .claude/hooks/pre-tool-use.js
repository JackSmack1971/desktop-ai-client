import { emitJson, readStdin, safeJsonParse } from "./helpers/io.js";
import { detectRisk } from "./helpers/risk-catalog.js";

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};
  const text = [
    event.command,
    event.tool,
    event.tool_name,
    event.input,
    event.content,
    input
  ]
    .filter(Boolean)
    .join(" ");

  const hits = detectRisk(text);
  if (hits.length > 0) {
    emitJson({
      status: "block",
      hook: "pre-tool-use",
      reason: "dangerous pattern detected",
      hits
    });
    process.exit(1);
    return;
  }

  emitJson({
    status: "allow",
    hook: "pre-tool-use",
    command: event.command || event.tool || null
  });
}

main().catch((error) => {
  emitJson({
    status: "error",
    hook: "pre-tool-use",
    message: error.message
  });
  process.exit(1);
});

