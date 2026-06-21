import { emitHookError, emitJson, readStdin, safeJsonParse } from "./helpers/io.js";

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};
  emitJson({
    hookSpecificOutput: {
      hookEventName: "Stop",
      additionalContext: [
        "Return a findings-first result with explicit evidence and remaining risk.",
        event.session_id ? `Session: ${event.session_id}` : null
      ]
        .filter(Boolean)
        .join(" ")
    }
  });
}

main().catch((error) => {
  emitHookError("Stop", error);
  console.error(error.stack || error.message);
  process.exit(2);
});
