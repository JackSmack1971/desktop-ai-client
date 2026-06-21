import { emitHookError, emitJson, readStdin, safeJsonParse } from "./helpers/io.js";
import { detectRisk, dangerousCommandPatterns, dangerousPathPatterns } from "./helpers/risk-catalog.js";

function emitDecision(permissionDecision, reason, updatedInput) {
  emitJson({
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision,
      ...(reason ? { permissionDecisionReason: reason } : {}),
      ...(updatedInput ? { updatedInput } : {})
    }
  });
}

function collectPathStrings(value) {
  const parts = [];
  const visit = (node, key) => {
    if (typeof node === "string") {
      if (!key || /path/i.test(key)) {
        parts.push(node);
      }
      return;
    }

    if (!node || typeof node !== "object") {
      return;
    }

    if (Array.isArray(node)) {
      for (const item of node) {
        visit(item, key);
      }
      return;
    }

    for (const [childKey, childValue] of Object.entries(node)) {
      visit(childValue, childKey);
    }
  };

  visit(value, "");
  return parts.join(" ");
}

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};

  const toolName = String(event.tool_name || "");
  const toolInput = event.tool_input && typeof event.tool_input === "object" ? event.tool_input : {};
  let text = "";
  let patterns = dangerousCommandPatterns;

  if (toolName === "Bash") {
    text = String(toolInput.command || toolInput.bash_command || "");
  } else if (toolName === "Write" || toolName === "Edit" || toolName === "MultiEdit" || toolName === "NotebookEdit") {
    text = collectPathStrings(toolInput);
    patterns = dangerousPathPatterns;
  } else {
    emitDecision("deny", `unsupported mutating tool: ${toolName || "<unknown>"}`);
    return;
  }

  const hits = detectRisk(text, patterns);
  if (hits.length > 0) {
    emitDecision("deny", `dangerous pattern detected: ${hits.join(", ")}`);
    return;
  }

  emitDecision("allow");
}

main().catch((error) => {
  emitHookError("PreToolUse", error, {
    permissionDecision: "deny",
    permissionDecisionReason: "hook execution failed",
  });
  console.error(error.stack || error.message);
  process.exit(2);
});
