export const CLIPBOARD_TEXT_MAX_CHARS = 1_000_000;

function charLength(value: string) {
  return [...value].length;
}

export function normalizeClipboardText(text: unknown): string {
  if (typeof text !== "string") {
    throw new Error("SEC_INVALID_INPUT: clipboard text must be a string");
  }
  const normalized = text.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: clipboard text is required");
  }
  if (charLength(normalized) > CLIPBOARD_TEXT_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: clipboard text is too long (max ${CLIPBOARD_TEXT_MAX_CHARS} chars)`
    );
  }
  return normalized;
}
