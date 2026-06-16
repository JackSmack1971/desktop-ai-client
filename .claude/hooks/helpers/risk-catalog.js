const dangerousPatterns = [
  /rm\s+-rf/i,
  /git\s+reset\s+--hard/i,
  /git\s+push\s+--force/i,
  /dropdb/i,
  /terraform\s+destroy/i,
  /kubectl\s+delete/i,
  /del\s+\/f\s+\/s\s+\/q/i,
  /\.env(\.|$)/i,
  /secrets?/i
];

function detectRisk(text) {
  const hits = [];
  const source = String(text || "");
  for (const pattern of dangerousPatterns) {
    if (pattern.test(source)) {
      hits.push(pattern.source);
    }
  }
  return hits;
}

export {
  detectRisk,
  dangerousPatterns,
};

