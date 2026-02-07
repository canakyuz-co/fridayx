import { useCallback, useEffect, useRef, useState } from "react";
import {
  editorApplyDelta,
  editorClose,
  editorFlushToDisk,
  editorOpen,
  readWorkspaceFile,
  writeWorkspaceFile,
} from "../../../services/tauri";
import { monacoLanguageFromPath } from "../../../utils/languageRegistry";

type EditorBuffer = {
  path: string;
  content: string;
  language: string | null;
  isDirty: boolean;
  isSaving: boolean;
  isLoading: boolean;
  error: string | null;
  isTruncated: boolean;
  rustBufferId: number | null;
  rustVersion: number | null;
  rustByteLen: number;
};

type UseEditorStateOptions = {
  workspaceId: string | null;
  availablePaths?: string[];
  filesReady?: boolean;
  onDidSave?: (path: string) => void;
};

type UseEditorStateResult = {
  openPaths: string[];
  activePath: string | null;
  buffersByPath: Record<string, EditorBuffer>;
  openFile: (path: string) => void;
  closeFile: (path: string) => void;
  setActivePath: (path: string) => void;
  updateContent: (path: string, value: string) => void;
  saveFile: (path: string) => void;
};

type RustBufferMeta = {
  bufferId: number;
  version: number;
  byteLen: number;
};

type TextPatch = {
  start: number;
  end: number;
  insertText: string;
};

function computeSinglePatch(previous: string, next: string): TextPatch | null {
  if (previous === next) {
    return null;
  }
  let start = 0;
  const minLength = Math.min(previous.length, next.length);
  while (start < minLength && previous.charCodeAt(start) === next.charCodeAt(start)) {
    start += 1;
  }
  let prevEnd = previous.length;
  let nextEnd = next.length;
  while (
    prevEnd > start &&
    nextEnd > start &&
    previous.charCodeAt(prevEnd - 1) === next.charCodeAt(nextEnd - 1)
  ) {
    prevEnd -= 1;
    nextEnd -= 1;
  }
  return {
    start,
    end: prevEnd,
    insertText: next.slice(start, nextEnd),
  };
}


export function useEditorState({
  workspaceId,
  availablePaths = [],
  filesReady = true,
  onDidSave,
}: UseEditorStateOptions): UseEditorStateResult {
  const [openPaths, setOpenPaths] = useState<string[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [buffersByPath, setBuffersByPath] = useState<Record<string, EditorBuffer>>({});
  const latestBuffersRef = useRef<Record<string, EditorBuffer>>({});
  const rustMetaByPathRef = useRef<Record<string, RustBufferMeta>>({});
  const rustSyncQueueRef = useRef<Record<string, Promise<void>>>({});
  const textEncoderRef = useRef(new TextEncoder());
  const hasRestoredRef = useRef(false);

  const getLastFileKey = useCallback(
    (id: string) => `codexmonitor.editorLastFile.${id}`,
    [],
  );

  const findReadmePath = useCallback((paths: string[]) => {
    if (!paths.length) {
      return null;
    }
    let best: { path: string; extensionWeight: number; depth: number; length: number } | null =
      null;
    for (const path of paths) {
      const name = path.split("/").pop() ?? path;
      const lower = name.toLowerCase();
      if (!lower.startsWith("readme")) {
        continue;
      }
      const isExactMd = lower === "readme.md";
      const isExactMdx = lower === "readme.mdx";
      const isExact = lower === "readme";
      const candidate = {
        path,
        extensionWeight: isExactMd ? 0 : isExactMdx ? 1 : isExact ? 2 : 3,
        depth: path.split("/").length,
        length: path.length,
      };
      if (
        !best ||
        candidate.extensionWeight < best.extensionWeight ||
        (candidate.extensionWeight === best.extensionWeight &&
          (candidate.depth < best.depth ||
            (candidate.depth === best.depth && candidate.length < best.length)))
      ) {
        best = candidate;
      }
    }
    return best?.path ?? null;
  }, []);

  const openFile = useCallback(
    (path: string) => {
      if (!workspaceId) {
        return;
      }
      setActivePath(path);
      setOpenPaths((prev) => (prev.includes(path) ? prev : [...prev, path]));
      setBuffersByPath((prev) => {
        if (prev[path]) {
          return prev;
        }
        return {
          ...prev,
          [path]: {
            path,
            content: "",
            language: monacoLanguageFromPath(path),
            isDirty: false,
            isSaving: false,
            isLoading: true,
            error: null,
            isTruncated: false,
            rustBufferId: null,
            rustVersion: null,
            rustByteLen: 0,
          },
        };
      });
      void (async () => {
        try {
          const response = await readWorkspaceFile(workspaceId, path);
          let rustBufferId: number | null = null;
          let rustVersion: number | null = null;
          let rustByteLen = 0;
          try {
            const snapshot = await editorOpen(workspaceId, path, response.content);
            rustBufferId = snapshot.bufferId;
            rustVersion = snapshot.version;
            rustByteLen = snapshot.byteLen;
            rustMetaByPathRef.current[path] = {
              bufferId: snapshot.bufferId,
              version: snapshot.version,
              byteLen: snapshot.byteLen,
            };
          } catch {
            // Keep local editing usable even if Rust core buffer init fails.
            delete rustMetaByPathRef.current[path];
          }
          setBuffersByPath((prev) => {
            const current = prev[path];
          if (!current) {
            return prev;
          }
          return {
            ...prev,
            [path]: {
              ...current,
              content: response.content,
              isLoading: false,
              error: null,
              isTruncated: response.truncated,
              rustBufferId,
              rustVersion,
              rustByteLen,
            },
          };
        });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          delete rustMetaByPathRef.current[path];
          setBuffersByPath((prev) => {
            const current = prev[path];
            if (!current) {
              return prev;
            }
            return {
              ...prev,
              [path]: {
                ...current,
                isLoading: false,
                error: message,
              },
            };
          });
        }
      })();
    },
    [workspaceId],
  );

  useEffect(() => {
    const buffers = latestBuffersRef.current;
    for (const buffer of Object.values(buffers)) {
      if (buffer.rustBufferId) {
        void editorClose(buffer.rustBufferId).catch(() => {});
      }
    }
    rustMetaByPathRef.current = {};
    rustSyncQueueRef.current = {};
    setOpenPaths([]);
    setActivePath(null);
    setBuffersByPath({});
    hasRestoredRef.current = false;
  }, [workspaceId]);

  useEffect(() => {
    latestBuffersRef.current = buffersByPath;
  }, [buffersByPath]);

  useEffect(() => {
    if (!workspaceId || !filesReady) {
      return;
    }
    if (hasRestoredRef.current) {
      return;
    }
    if (openPaths.length > 0 || activePath) {
      hasRestoredRef.current = true;
      return;
    }
    const storedPath =
      typeof window === "undefined"
        ? null
        : window.localStorage.getItem(getLastFileKey(workspaceId));
    const storedIsValid = storedPath ? availablePaths.includes(storedPath) : false;
    const readmePath = storedIsValid ? null : findReadmePath(availablePaths);
    const nextPath = storedIsValid ? storedPath : readmePath;
    hasRestoredRef.current = true;
    if (nextPath) {
      openFile(nextPath);
    }
  }, [
    activePath,
    availablePaths,
    filesReady,
    findReadmePath,
    getLastFileKey,
    openFile,
    openPaths.length,
    workspaceId,
  ]);

  useEffect(() => {
    if (!workspaceId || !activePath || typeof window === "undefined") {
      return;
    }
    window.localStorage.setItem(getLastFileKey(workspaceId), activePath);
  }, [activePath, getLastFileKey, workspaceId]);

  const closeFile = useCallback((path: string) => {
    const buffer = latestBuffersRef.current[path];
    if (buffer?.rustBufferId) {
      void editorClose(buffer.rustBufferId).catch(() => {});
    }
    delete rustMetaByPathRef.current[path];
    delete rustSyncQueueRef.current[path];
    setOpenPaths((prev) => {
      const next = prev.filter((entry) => entry !== path);
      setActivePath((current) => {
        if (current !== path) {
          return current;
        }
        return next[next.length - 1] ?? null;
      });
      return next;
    });
    setBuffersByPath((prev) => {
      const next = { ...prev };
      delete next[path];
      return next;
    });
  }, []);

  const updateContent = useCallback((path: string, value: string) => {
    const toByteOffset = (text: string, charOffset: number) =>
      textEncoderRef.current.encode(text.slice(0, charOffset)).length;
    const toByteLen = (text: string) => textEncoderRef.current.encode(text).length;
    const currentBuffer = latestBuffersRef.current[path];
    if (!currentBuffer || currentBuffer.isLoading) {
      return;
    }
    const shouldSyncRust =
      currentBuffer.rustBufferId != null && currentBuffer.rustVersion != null;
    const previousContent = currentBuffer.content;

    setBuffersByPath((prev) => {
      const current = prev[path];
      if (!current || current.isLoading) {
        return prev;
      }
      return {
        ...prev,
        [path]: {
          ...current,
          content: value,
          isDirty: true,
        },
      };
    });

    if (!shouldSyncRust || currentBuffer.rustBufferId == null) {
      return;
    }
    const patch = computeSinglePatch(previousContent, value);
    if (!patch) {
      return;
    }
    const startByte = toByteOffset(previousContent, patch.start);
    const endByte = toByteOffset(previousContent, patch.end);
    const nextByteLen = toByteLen(value);
    const bufferId = currentBuffer.rustBufferId;
    const queue = rustSyncQueueRef.current[path] ?? Promise.resolve();
    rustSyncQueueRef.current[path] = queue
      .then(async () => {
        const meta = rustMetaByPathRef.current[path];
        if (!meta || meta.bufferId !== bufferId) {
          return;
        }
        const result = await editorApplyDelta(
          meta.bufferId,
          meta.version,
          startByte,
          endByte,
          patch.insertText,
        );
        rustMetaByPathRef.current[path] = {
          ...meta,
          version: result.version,
          byteLen: nextByteLen,
        };
        setBuffersByPath((prev) => {
          const current = prev[path];
          if (!current || current.rustBufferId !== meta.bufferId) {
            return prev;
          }
          return {
            ...prev,
            [path]: {
              ...current,
              rustVersion: result.version,
              rustByteLen: nextByteLen,
            },
          };
        });
      })
      .catch(() => {
        delete rustMetaByPathRef.current[path];
        setBuffersByPath((prev) => {
          const current = prev[path];
          if (!current) {
            return prev;
          }
          return {
            ...prev,
            [path]: {
              ...current,
              rustVersion: null,
              rustByteLen: 0,
              rustBufferId: null,
            },
          };
        });
      });
  }, []);

  const saveFile = useCallback(
    (path: string) => {
      if (!workspaceId) {
        return;
      }
      const buffer = buffersByPath[path];
      if (!buffer || buffer.isLoading || buffer.isSaving || buffer.isTruncated) {
        return;
      }
      setBuffersByPath((prev) => {
        const current = prev[path];
        if (!current) {
          return prev;
        }
        return {
          ...prev,
          [path]: {
            ...current,
            isSaving: true,
            error: null,
          },
        };
      });
      void (async () => {
        try {
          await (rustSyncQueueRef.current[path] ?? Promise.resolve());
          const latestBuffer = latestBuffersRef.current[path] ?? buffer;
          const rustMeta = rustMetaByPathRef.current[path];
          if (rustMeta) {
            await editorFlushToDisk(rustMeta.bufferId);
            setBuffersByPath((prev) => {
              const current = prev[path];
              if (!current) {
                return prev;
              }
              return {
                ...prev,
                [path]: {
                  ...current,
                  isDirty: false,
                  isSaving: false,
                  error: null,
                  rustVersion: rustMeta.version,
                  rustByteLen: rustMeta.byteLen,
                },
              };
            });
          } else {
            await writeWorkspaceFile(workspaceId, path, latestBuffer.content);
            setBuffersByPath((prev) => {
              const current = prev[path];
              if (!current) {
                return prev;
              }
              return {
                ...prev,
                [path]: {
                  ...current,
                  isDirty: false,
                  isSaving: false,
                  error: null,
                },
              };
            });
          }
          onDidSave?.(path);
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setBuffersByPath((prev) => {
            const current = prev[path];
            if (!current) {
              return prev;
            }
            return {
              ...prev,
              [path]: {
                ...current,
                isSaving: false,
                error: message,
              },
            };
          });
        }
      })();
    },
    [workspaceId, buffersByPath, onDidSave],
  );

  return {
    openPaths,
    activePath,
    buffersByPath,
    openFile,
    closeFile,
    setActivePath,
    updateContent,
    saveFile,
  };
}
