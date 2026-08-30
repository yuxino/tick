export function displayPath(path: string, homeDirectory: string) {
  const home = homeDirectory.replace(/[\\/]+$/, "");
  if (!path || !home) return path;

  const caseInsensitive = /^[A-Za-z]:[\\/]/.test(home);
  const comparablePath = caseInsensitive ? path.replace(/\\/g, "/").toLowerCase() : path;
  const comparableHome = caseInsensitive ? home.replace(/\\/g, "/").toLowerCase() : home;
  if (comparablePath === comparableHome) return "~";
  if (!comparablePath.startsWith(comparableHome)) return path;

  const boundary = path.charAt(home.length);
  return boundary === "/" || boundary === "\\" ? `~${path.slice(home.length)}` : path;
}
