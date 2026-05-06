export function friendlyError(err: unknown) {
  const message = err instanceof Error ? err.message : String(err);

  if (
    message.includes("Cannot read properties of undefined (reading 'invoke')") ||
    message.includes("__TAURI_INTERNALS__")
  ) {
    return "当前浏览器环境无法访问 Tauri 后端，请使用 npm run tauri dev 打开桌面应用。";
  }

  return message.replace(/^Error:\s*/, "");
}
