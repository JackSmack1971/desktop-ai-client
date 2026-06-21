import { emitHookError, emitJson, readStdin, safeJsonParse } from "./helpers/io.js";

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};
  emitJson({
    hookSpecificOutput: {
      hookEventName: "PostToolUse",
      additionalContext: [
        "Verify the changed boundary before broadening scope.",
        event.tool_name ? `Tool: ${event.tool_name}` : null,
        event.tool_response && typeof event.tool_response === "object" && "success" in event.tool_response
          ? `Success: ${String(event.tool_response.success)}`
          : null
      ]
        .filter(Boolean)
        .join(" ")
    }
  });
}

main().catch((error) => {
  emitHookError("PostToolUse", error);
  console.error(error.stack || error.message);
  process.exit(2);
});
