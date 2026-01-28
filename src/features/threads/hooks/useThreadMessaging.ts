import { useCallback } from "react";
import type { Dispatch, MutableRefObject } from "react";
import * as Sentry from "@sentry/react";
import type {
  AccessMode,
  CustomPromptOption,
  DebugEntry,
  OtherAiProvider,
  WorkspaceInfo,
} from "../../../types";
import {
  sendUserMessage as sendUserMessageService,
  startReview as startReviewService,
  interruptTurn as interruptTurnService,
  sendClaudeMessage,
  sendClaudeCliMessage,
  type ClaudeMessage,
  type ClaudeRateLimits,
  type ClaudeUsage,
} from "../../../services/tauri";
import { expandCustomPromptText } from "../../../utils/customPrompts";
import {
  asString,
  extractRpcErrorMessage,
  parseReviewTarget,
} from "../utils/threadNormalize";
import type { ThreadAction, ThreadState } from "./useThreadsReducer";

type SendMessageOptions = {
  skipPromptExpansion?: boolean;
  model?: string | null;
  effort?: string | null;
  collaborationMode?: Record<string, unknown> | null;
  accessMode?: AccessMode;
};

type UseThreadMessagingOptions = {
  activeWorkspace: WorkspaceInfo | null;
  activeThreadId: string | null;
  accessMode?: "read-only" | "current" | "full-access";
  model?: string | null;
  effort?: string | null;
  collaborationMode?: Record<string, unknown> | null;
  steerEnabled: boolean;
  customPrompts: CustomPromptOption[];
  otherAiProviders: OtherAiProvider[];
  threadStatusById: ThreadState["threadStatusById"];
  activeTurnIdByThread: ThreadState["activeTurnIdByThread"];
  pendingInterruptsRef: MutableRefObject<Set<string>>;
  dispatch: Dispatch<ThreadAction>;
  getCustomName: (workspaceId: string, threadId: string) => string | undefined;
  markProcessing: (threadId: string, isProcessing: boolean) => void;
  markReviewing: (threadId: string, isReviewing: boolean) => void;
  setActiveTurnId: (threadId: string, turnId: string | null) => void;
  recordThreadActivity: (
    workspaceId: string,
    threadId: string,
    timestamp?: number,
  ) => void;
  safeMessageActivity: () => void;
  onDebug?: (entry: DebugEntry) => void;
  onClaudeRateLimits?: (limits: ClaudeRateLimits) => void;
  onClaudeUsage?: (usage: ClaudeUsage) => void;
  pushThreadErrorMessage: (threadId: string, message: string) => void;
  ensureThreadForActiveWorkspace: () => Promise<string | null>;
};

export function useThreadMessaging({
  activeWorkspace,
  activeThreadId,
  accessMode,
  model,
  effort,
  collaborationMode,
  steerEnabled,
  customPrompts,
  otherAiProviders,
  threadStatusById,
  activeTurnIdByThread,
  pendingInterruptsRef,
  dispatch,
  getCustomName,
  markProcessing,
  markReviewing,
  setActiveTurnId,
  recordThreadActivity,
  safeMessageActivity,
  onDebug,
  onClaudeRateLimits,
  onClaudeUsage,
  pushThreadErrorMessage,
  ensureThreadForActiveWorkspace,
}: UseThreadMessagingOptions) {
  const sendMessageToThread = useCallback(
    async (
      workspace: WorkspaceInfo,
      threadId: string,
      text: string,
      images: string[] = [],
      options?: SendMessageOptions,
    ) => {
      const messageText = text.trim();
      if (!messageText && images.length === 0) {
        return;
      }
      let finalText = messageText;
      if (!options?.skipPromptExpansion) {
        const promptExpansion = expandCustomPromptText(messageText, customPrompts);
        if (promptExpansion && "error" in promptExpansion) {
          pushThreadErrorMessage(threadId, promptExpansion.error);
          safeMessageActivity();
          return;
        }
        finalText = promptExpansion?.expanded ?? messageText;
      }
      const resolvedModel =
        options?.model !== undefined ? options.model : model;
      const resolvedEffort =
        options?.effort !== undefined ? options.effort : effort;
      const resolvedCollaborationMode =
        options?.collaborationMode !== undefined
          ? options.collaborationMode
          : collaborationMode;
      const sanitizedCollaborationMode =
        resolvedCollaborationMode &&
        typeof resolvedCollaborationMode === "object" &&
        "settings" in resolvedCollaborationMode
          ? resolvedCollaborationMode
          : null;
      const resolvedAccessMode =
        options?.accessMode !== undefined ? options.accessMode : accessMode;

      const wasProcessing =
        (threadStatusById[threadId]?.isProcessing ?? false) && steerEnabled;
      if (wasProcessing) {
        const optimisticText = finalText || (images.length > 0 ? "[image]" : "");
        if (optimisticText) {
          dispatch({
            type: "upsertItem",
            workspaceId: workspace.id,
            threadId,
            item: {
              id: `optimistic-user-${Date.now()}-${Math.random()
                .toString(36)
                .slice(2, 8)}`,
              kind: "message",
              role: "user",
              text: optimisticText,
            },
            hasCustomName: Boolean(getCustomName(workspace.id, threadId)),
          });
        }
      }
      Sentry.metrics.count("prompt_sent", 1, {
        attributes: {
          workspace_id: workspace.id,
          thread_id: threadId,
          has_images: images.length > 0 ? "true" : "false",
          text_length: String(finalText.length),
          model: resolvedModel ?? "unknown",
          effort: resolvedEffort ?? "unknown",
          collaboration_mode: sanitizedCollaborationMode ?? "unknown",
        },
      });
      const timestamp = Date.now();
      recordThreadActivity(workspace.id, threadId, timestamp);
      dispatch({
        type: "setThreadTimestamp",
        workspaceId: workspace.id,
        threadId,
        timestamp,
      });
      markProcessing(threadId, true);
      safeMessageActivity();
      onDebug?.({
        id: `${Date.now()}-client-turn-start`,
        timestamp: Date.now(),
        source: "client",
        label: "turn/start",
        payload: {
          workspaceId: workspace.id,
          threadId,
          text: finalText,
          images,
          model: resolvedModel,
          effort: resolvedEffort,
          collaborationMode: sanitizedCollaborationMode,
        },
      });
      try {
        // Check if this is a Claude model (format: "providerId:model-name")
        const isOtherAiModel = resolvedModel?.includes(":") ?? false;
        const colonIndex = resolvedModel?.indexOf(":") ?? -1;
        const providerId = isOtherAiModel ? resolvedModel!.slice(0, colonIndex) : null;
        const provider = providerId ? otherAiProviders.find((p) => p.id === providerId) : null;

        if (provider && provider.provider === "claude") {
          // Claude provider - use CLI or API
          const useCli = Boolean(provider.command);
          const useApi = Boolean(provider.apiKey) && !useCli;

          if (!useCli && !useApi) {
            markProcessing(threadId, false);
            pushThreadErrorMessage(
              threadId,
              "Claude not configured. Set CLI command or API key in Settings > Other AI."
            );
            safeMessageActivity();
            return;
          }

          // Add user message to thread immediately
          dispatch({
            type: "upsertItem",
            workspaceId: workspace.id,
            threadId,
            item: {
              id: `user-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
              kind: "message",
              role: "user",
              text: finalText,
            },
            hasCustomName: Boolean(getCustomName(workspace.id, threadId)),
          });

          const assistantMessageId = `assistant-${Date.now()}-${Math.random()
            .toString(36)
            .slice(2, 8)}`;

          if (useCli) {
            // Use Claude CLI
            await sendClaudeCliMessage(
              provider.command!,
              provider.args,
              finalText,
              workspace.path,
              {
                onInit: (sessionId, model) => {
                  onDebug?.({
                    id: `${Date.now()}-claude-cli-init`,
                    timestamp: Date.now(),
                    source: "client",
                    label: "claude-cli/init",
                    payload: { sessionId, model },
                  });
                },
                onContent: (text) => {
                  dispatch({
                    type: "upsertItem",
                    workspaceId: workspace.id,
                    threadId,
                    item: {
                      id: assistantMessageId,
                      kind: "message",
                      role: "assistant",
                      text,
                    },
                    hasCustomName: Boolean(getCustomName(workspace.id, threadId)),
                  });
                  safeMessageActivity();
                },
                onComplete: (_text, usage) => {
                  if (usage) {
                    onClaudeUsage?.({
                      inputTokens: usage.inputTokens,
                      outputTokens: usage.outputTokens,
                    });
                  }
                  markProcessing(threadId, false);
                  safeMessageActivity();
                },
                onError: (error) => {
                  markProcessing(threadId, false);
                  pushThreadErrorMessage(threadId, error);
                  safeMessageActivity();
                },
              }
            );
          } else {
            // Use Claude API
            const modelName = resolvedModel!.slice(colonIndex + 1);
            const claudeMessages: ClaudeMessage[] = [
              { role: "user", content: finalText },
            ];

            let accumulatedText = "";

            await sendClaudeMessage(provider.apiKey!, modelName, claudeMessages, {
              onContent: (text) => {
                accumulatedText += text;
                dispatch({
                  type: "upsertItem",
                  workspaceId: workspace.id,
                  threadId,
                  item: {
                    id: assistantMessageId,
                    kind: "message",
                    role: "assistant",
                    text: accumulatedText,
                  },
                  hasCustomName: Boolean(getCustomName(workspace.id, threadId)),
                });
                safeMessageActivity();
              },
              onComplete: (_fullText, usage) => {
                if (usage) {
                  onClaudeUsage?.(usage);
                }
                markProcessing(threadId, false);
                safeMessageActivity();
              },
              onRateLimits: (limits) => {
                onClaudeRateLimits?.(limits);
              },
              onError: (error) => {
                markProcessing(threadId, false);
                pushThreadErrorMessage(threadId, error);
                safeMessageActivity();
              },
            });
          }

          onDebug?.({
            id: `${Date.now()}-claude-message-sent`,
            timestamp: Date.now(),
            source: "client",
            label: "claude/message",
            payload: { model: resolvedModel, textLength: finalText.length },
          });

          return;
        }

        // Existing Codex flow
        const response =
          (await sendUserMessageService(
            workspace.id,
            threadId,
            finalText,
            {
              model: resolvedModel,
              effort: resolvedEffort,
              collaborationMode: sanitizedCollaborationMode,
              accessMode: resolvedAccessMode,
              images,
            },
          )) as Record<string, unknown>;
        onDebug?.({
          id: `${Date.now()}-server-turn-start`,
          timestamp: Date.now(),
          source: "server",
          label: "turn/start response",
          payload: response,
        });
        const rpcError = extractRpcErrorMessage(response);
        if (rpcError) {
          markProcessing(threadId, false);
          setActiveTurnId(threadId, null);
          pushThreadErrorMessage(threadId, `Turn failed to start: ${rpcError}`);
          safeMessageActivity();
          return;
        }
        const result = (response?.result ?? response) as Record<string, unknown>;
        const turn = (result?.turn ?? response?.turn ?? null) as
          | Record<string, unknown>
          | null;
        const turnId = asString(turn?.id ?? "");
        if (!turnId) {
          markProcessing(threadId, false);
          setActiveTurnId(threadId, null);
          pushThreadErrorMessage(threadId, "Turn failed to start.");
          safeMessageActivity();
          return;
        }
        setActiveTurnId(threadId, turnId);
      } catch (error) {
        markProcessing(threadId, false);
        setActiveTurnId(threadId, null);
        onDebug?.({
          id: `${Date.now()}-client-turn-start-error`,
          timestamp: Date.now(),
          source: "error",
          label: "turn/start error",
          payload: error instanceof Error ? error.message : String(error),
        });
        pushThreadErrorMessage(
          threadId,
          error instanceof Error ? error.message : String(error),
        );
        safeMessageActivity();
      }
    },
    [
      accessMode,
      collaborationMode,
      customPrompts,
      dispatch,
      effort,
      getCustomName,
      markProcessing,
      model,
      onClaudeRateLimits,
      onClaudeUsage,
      onDebug,
      otherAiProviders,
      pushThreadErrorMessage,
      recordThreadActivity,
      safeMessageActivity,
      setActiveTurnId,
      steerEnabled,
      threadStatusById,
    ],
  );

  const sendUserMessage = useCallback(
    async (text: string, images: string[] = []) => {
      if (!activeWorkspace) {
        return;
      }
      const messageText = text.trim();
      if (!messageText && images.length === 0) {
        return;
      }
      const promptExpansion = expandCustomPromptText(messageText, customPrompts);
      if (promptExpansion && "error" in promptExpansion) {
        if (activeThreadId) {
          pushThreadErrorMessage(activeThreadId, promptExpansion.error);
          safeMessageActivity();
        } else {
          onDebug?.({
            id: `${Date.now()}-client-prompt-expand-error`,
            timestamp: Date.now(),
            source: "error",
            label: "prompt/expand error",
            payload: promptExpansion.error,
          });
        }
        return;
      }
      const finalText = promptExpansion?.expanded ?? messageText;
      const threadId = await ensureThreadForActiveWorkspace();
      if (!threadId) {
        return;
      }
      await sendMessageToThread(activeWorkspace, threadId, finalText, images, {
        skipPromptExpansion: true,
      });
    },
    [
      activeThreadId,
      activeWorkspace,
      customPrompts,
      ensureThreadForActiveWorkspace,
      onDebug,
      pushThreadErrorMessage,
      safeMessageActivity,
      sendMessageToThread,
    ],
  );

  const sendUserMessageToThread = useCallback(
    async (
      workspace: WorkspaceInfo,
      threadId: string,
      text: string,
      images: string[] = [],
      options?: SendMessageOptions,
    ) => {
      await sendMessageToThread(workspace, threadId, text, images, options);
    },
    [sendMessageToThread],
  );

  const interruptTurn = useCallback(async () => {
    if (!activeWorkspace || !activeThreadId) {
      return;
    }
    const activeTurnId = activeTurnIdByThread[activeThreadId] ?? null;
    const turnId = activeTurnId ?? "pending";
    markProcessing(activeThreadId, false);
    setActiveTurnId(activeThreadId, null);
    dispatch({
      type: "addAssistantMessage",
      threadId: activeThreadId,
      text: "Session stopped.",
    });
    if (!activeTurnId) {
      pendingInterruptsRef.current.add(activeThreadId);
    }
    onDebug?.({
      id: `${Date.now()}-client-turn-interrupt`,
      timestamp: Date.now(),
      source: "client",
      label: "turn/interrupt",
      payload: {
        workspaceId: activeWorkspace.id,
        threadId: activeThreadId,
        turnId,
        queued: !activeTurnId,
      },
    });
    try {
      const response = await interruptTurnService(
        activeWorkspace.id,
        activeThreadId,
        turnId,
      );
      onDebug?.({
        id: `${Date.now()}-server-turn-interrupt`,
        timestamp: Date.now(),
        source: "server",
        label: "turn/interrupt response",
        payload: response,
      });
    } catch (error) {
      onDebug?.({
        id: `${Date.now()}-client-turn-interrupt-error`,
        timestamp: Date.now(),
        source: "error",
        label: "turn/interrupt error",
        payload: error instanceof Error ? error.message : String(error),
      });
    }
  }, [
    activeThreadId,
    activeTurnIdByThread,
    activeWorkspace,
    dispatch,
    markProcessing,
    onDebug,
    pendingInterruptsRef,
    setActiveTurnId,
  ]);

  const startReview = useCallback(
    async (text: string) => {
      if (!activeWorkspace || !text.trim()) {
        return;
      }
      const threadId = await ensureThreadForActiveWorkspace();
      if (!threadId) {
        return;
      }

      const target = parseReviewTarget(text);
      markProcessing(threadId, true);
      markReviewing(threadId, true);
      safeMessageActivity();
      onDebug?.({
        id: `${Date.now()}-client-review-start`,
        timestamp: Date.now(),
        source: "client",
        label: "review/start",
        payload: {
          workspaceId: activeWorkspace.id,
          threadId,
          target,
        },
      });
      try {
        const response = await startReviewService(
          activeWorkspace.id,
          threadId,
          target,
          "inline",
        );
        onDebug?.({
          id: `${Date.now()}-server-review-start`,
          timestamp: Date.now(),
          source: "server",
          label: "review/start response",
          payload: response,
        });
        const rpcError = extractRpcErrorMessage(response);
        if (rpcError) {
          markProcessing(threadId, false);
          markReviewing(threadId, false);
          setActiveTurnId(threadId, null);
          pushThreadErrorMessage(threadId, `Review failed to start: ${rpcError}`);
          safeMessageActivity();
          return;
        }
      } catch (error) {
        markProcessing(threadId, false);
        markReviewing(threadId, false);
        onDebug?.({
          id: `${Date.now()}-client-review-start-error`,
          timestamp: Date.now(),
          source: "error",
          label: "review/start error",
          payload: error instanceof Error ? error.message : String(error),
        });
        pushThreadErrorMessage(
          threadId,
          error instanceof Error ? error.message : String(error),
        );
        safeMessageActivity();
      }
    },
    [
      activeWorkspace,
      ensureThreadForActiveWorkspace,
      markProcessing,
      markReviewing,
      onDebug,
      pushThreadErrorMessage,
      safeMessageActivity,
      setActiveTurnId,
    ],
  );

  return {
    interruptTurn,
    sendUserMessage,
    sendUserMessageToThread,
    startReview,
  };
}
