const dangerousCommandPatterns = [
  /rm\s+-rf/i,
  /git\s+reset\s+--hard/i,
  /git\s+push\s+--force/i,
  /dropdb/i,
  /terraform\s+destroy/i,
  /kubectl\s+delete/i,
  /del\s+\/f\s+\/s\s+\/q/i,
  /secrets?/i
];

const dangerousPathPatterns = [
  /(?:^|[\\/])\.env(?:$|[\\/]|\.local$|\.production$|\.staging$|\.development$)/i
];

function detectRisk(text, patterns = dangerousCommandPatterns) {
  const hits = [];
  const source = String(text || "");
  for (const pattern of patterns) {
    if (pattern.test(source)) {
      hits.push(pattern.source);
    }
  }
  return hits;
}

export {
  detectRisk,
  dangerousCommandPatterns,
  dangerousPathPatterns,
};
