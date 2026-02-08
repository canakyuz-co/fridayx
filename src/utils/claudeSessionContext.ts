import type { ConversationItem, WorkspaceInfo } from "../types";

const MAX_CONTEXT_CHARS = 6000;

type ContextAction = {
  kind: string;
  summary: string;
};

function basename(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}

function collectActions(items: ConversationItem[]): ContextAction[] {
  const actions: ContextAction[] = [];
  for (const item of items) {
    switch (item.kind) {
      case "tool": {
        if (item.toolType === "commandExecution") {
          const cmd = item.title.replace(/^Command:\s*/i, "").trim();
          const shortCmd = cmd.length > 80 ? `${cmd.slice(0, 80)}…` : cmd;
          actions.push({ kind: "command", summary: shortCmd });
        } else if (item.toolType === "fileChange") {
          const files = (item.changes ?? []).map((c) => basename(c.path));
          if (files.length > 0) {
            actions.push({
              kind: "edit",
              summary: files.length === 1
                ? files[0]
                : `${files[0]} +${files.length - 1} files`,
            });
          }
        } else if (item.toolType === "webSearch") {
          actions.push({ kind: "search", summary: item.detail || "web" });
        } else {
          const label = item.title.replace(/^Tool:\s*/i, "").trim();
          if (label) {
            actions.push({ kind: "tool", summary: label.slice(0, 60) });
          }
        }
        break;
      }
      case "diff": {
        actions.push({ kind: "diff", summary: item.title });
        break;
      }
      case "review": {
        actions.push({
          kind: "review",
          summary: item.state === "completed" ? "completed" : "started",
        });
        break;
      }
      case "explore": {
        const labels = item.entries.map((e) => e.label).slice(0, 3);
        actions.push({ kind: "explore", summary: labels.join(", ") });
        break;
      }
      default:
        break;
    }
  }
  return actions;
}

function summarizeMessages(items: ConversationItem[]): string[] {
  const summaries: string[] = [];
  for (const item of items) {
    if (item.kind !== "message") continue;
    const text = item.text.trim();
    if (!text) continue;
    const firstLine = text.split("\n")[0].trim();
    const short = firstLine.length > 120 ? `${firstLine.slice(0, 120)}…` : firstLine;
    summaries.push(`${item.role === "user" ? "User" : "Assistant"}: ${short}`);
  }
  return summaries;
}

function formatActions(actions: ContextAction[]): string {
  if (actions.length === 0) return "";
  const deduplicated: ContextAction[] = [];
  const seen = new Set<string>();
  for (const action of actions) {
    const key = `${action.kind}:${action.summary}`;
    if (!seen.has(key)) {
      seen.add(key);
      deduplicated.push(action);
    }
  }
  const lines = deduplicated.slice(-20).map((a) => `- [${a.kind}] ${a.summary}`);
  return lines.join("\n");
}

export function buildClaudeSessionContext(
  workspace: WorkspaceInfo,
  items: ConversationItem[],
): string {
  const parts: string[] = [];

  // Project info
  const projectName = workspace.name || basename(workspace.path);
  parts.push(`Project: ${projectName}`);
  parts.push(`Path: ${workspace.path}`);
  if (workspace.worktree?.branch) {
    parts.push(`Branch: ${workspace.worktree.branch}`);
  }
  if (workspace.settings?.gitRoot) {
    parts.push(`Git root: ${workspace.settings.gitRoot}`);
  }

  // Conversation summary
  if (items.length > 0) {
    const messageSummaries = summarizeMessages(items);
    const actions = collectActions(items);
    const actionText = formatActions(actions);

    if (messageSummaries.length > 0 || actionText) {
      parts.push("");
      parts.push("Previous conversation context:");
    }

    if (actionText) {
      parts.push("");
      parts.push("Actions taken:");
      parts.push(actionText);
    }

    // Include last few message exchanges for continuity
    if (messageSummaries.length > 0) {
      const recentMessages = messageSummaries.slice(-8);
      parts.push("");
      parts.push("Recent messages:");
      for (const msg of recentMessages) {
        parts.push(`- ${msg}`);
      }
    }
  }

  parts.push("");
  parts.push("Continue this development session. You have full context from the previous conversation.");

  let result = parts.join("\n");
  if (result.length > MAX_CONTEXT_CHARS) {
    result = result.slice(0, MAX_CONTEXT_CHARS - 3) + "…";
  }
  return result;
}
