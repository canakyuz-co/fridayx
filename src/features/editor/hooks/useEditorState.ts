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
          } catch {
            // Keep local editing usable even if Rust core buffer init fails.
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
          const contentByteLen = new TextEncoder().encode(buffer.content).length;
          if (buffer.rustBufferId && buffer.rustVersion != null) {
            const delta = await editorApplyDelta(
              buffer.rustBufferId,
              buffer.rustVersion,
              0,
              buffer.rustByteLen,
              buffer.content,
            );
            await editorFlushToDisk(buffer.rustBufferId);
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
                  rustVersion: delta.version,
                  rustByteLen: contentByteLen,
                },
              };
            });
          } else {
            await writeWorkspaceFile(workspaceId, path, buffer.content);
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
