import { toast } from "sonner";
import type { ProviderEditorPayloadBuildError } from "./providerEditorActionContext";

function toastFirstSchemaIssue(
  mode: "create" | "edit",
  issues: Array<{ path: Array<PropertyKey>; message: string }>
) {
  const orderedFields = [
    "name",
    ...(mode === "create" ? (["api_key"] as const) : []),
    "cost_multiplier",
    "limit_5h_usd",
    "limit_daily_usd",
    "limit_weekly_usd",
    "limit_monthly_usd",
    "limit_total_usd",
    "daily_reset_time",
  ];

  const messageByField = new Map<string, string>();
  for (const issue of issues) {
    const firstSegment = issue.path[0];
    if (typeof firstSegment !== "string") continue;
    if (!messageByField.has(firstSegment)) {
      messageByField.set(firstSegment, issue.message);
    }
  }

  for (const field of orderedFields) {
    const maybeMessage = messageByField.get(field);
    if (maybeMessage) {
      toast(maybeMessage);
      return;
    }
  }

  const fallback = issues.find((issue) => Boolean(issue.message));
  if (fallback) {
    toast(fallback.message);
  }
}

export function presentProviderEditorPayloadBuildError(
  mode: "create" | "edit",
  error: ProviderEditorPayloadBuildError
) {
  if (error.kind === "message") {
    toast(error.message);
    return;
  }

  toastFirstSchemaIssue(mode, error.issues);
}
