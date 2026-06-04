import { writeDesktopClipboardText } from "./desktop/clipboard";
import { normalizeClipboardText } from "./clipboardText";

async function copyTextFallback(text: string) {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  document.body.appendChild(textarea);

  try {
    textarea.focus();
    textarea.select();
    const copied = document.execCommand("copy");
    if (!copied) throw new Error("CLIPBOARD_COPY_FAILED");
  } finally {
    document.body.removeChild(textarea);
  }
}

export async function copyText(text: string) {
  const normalizedText = normalizeClipboardText(text);

  try {
    await writeDesktopClipboardText(normalizedText);
    return;
  } catch {
    // fallback below
  }

  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(normalizedText);
      return;
    }
  } catch {
    // fallback below
  }

  await copyTextFallback(normalizedText);
}
