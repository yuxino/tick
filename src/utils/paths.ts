export function displayPath(path: string) {
  return path.replace(/^\/Users\/[^/]+(?=\/)/, "~");
}
