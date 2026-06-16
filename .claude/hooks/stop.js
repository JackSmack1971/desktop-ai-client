import { emitJson, readStdin, safeJsonParse } from "./helpers/io.js";

async function main() {
  const input = await readStdin();
  const event = safeJsonParse(input) || {};
  emitJson({
    status: "ok",
    hook: "stop",
    session: event.session || null,
    summary: "Return a findings-first result with explicit evidence and remaining risk."
  });
}

main().catch((error) => {
  emitJson({
    status: "error",
    hook: "stop",
    message: error.message
  });
  process.exit(1);
});

