import { emitJson, readStdin, safeJsonParse } from "./helpers/io.js";

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};
  emitJson({
    status: "ok",
    hook: "post-tool-use",
    command: event.command || event.tool || null,
    nextStep: "verify any changed boundary before broadening scope"
  });
}

main().catch((error) => {
  emitJson({
    status: "error",
    hook: "post-tool-use",
    message: error.message
  });
  process.exit(1);
});

