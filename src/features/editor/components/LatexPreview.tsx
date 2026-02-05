import { useEffect, useRef, useState } from "react";
import type { LatexCompileDiagnostic } from "../../../services/tauri";
import { latexCompile } from "../../../services/tauri";

type LatexPreviewProps = {
  workspaceId: string;
  path: string;
  source: string;
};

function base64ToObjectUrl(base64: string): string {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  const blob = new Blob([bytes], { type: "application/pdf" });
  return URL.createObjectURL(blob);
}

export function LatexPreview({ workspaceId, path, source }: LatexPreviewProps) {
  const [status, setStatus] = useState<"idle" | "compiling" | "ready" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<LatexCompileDiagnostic[]>([]);
  const [log, setLog] = useState<string>("");
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const urlRef = useRef<string | null>(null);

  useEffect(() => {
    // Debounce compile to keep typing smooth.
    setStatus("compiling");
    setError(null);

    const handle = window.setTimeout(() => {
      latexCompile(workspaceId, path, source)
        .then((res) => {
          setDiagnostics(res.diagnostics ?? []);
          setLog(res.log ?? "");

          const nextUrl = base64ToObjectUrl(res.pdfBase64);
          if (urlRef.current) {
            URL.revokeObjectURL(urlRef.current);
          }
          urlRef.current = nextUrl;
          setPdfUrl(nextUrl);
          setStatus("ready");
        })
        .catch((err: unknown) => {
          const message =
            err instanceof Error ? err.message : typeof err === "string" ? err : "Derleme hatasi";
          setError(message);
          setStatus("error");
        });
    }, 450);

    return () => window.clearTimeout(handle);
  }, [workspaceId, path, source]);

  useEffect(() => {
    return () => {
      if (urlRef.current) {
        URL.revokeObjectURL(urlRef.current);
      }
    };
  }, []);

  return (
    <div className="editor-latex-preview">
      <div className="editor-latex-toolbar">
        <span className="editor-latex-status">
          {status === "compiling" ? "Derleniyor..." : status === "ready" ? "Hazir" : "Hata"}
        </span>
        {diagnostics.length ? (
          <span className="editor-latex-pill">{diagnostics.length} tanilama</span>
        ) : null}
      </div>

      {error ? <div className="editor-latex-error">{error}</div> : null}

      {diagnostics.length ? (
        <div className="editor-latex-diagnostics" role="list">
          {diagnostics.slice(0, 8).map((d, idx) => (
            <div key={`${idx}-${d.message}`} className="editor-latex-diagnostic" role="listitem">
              <span className={`editor-latex-diag-level level-${d.level}`}>{d.level}</span>
              <span className="editor-latex-diag-message">
                {d.line ? `L${d.line}: ` : ""}
                {d.message}
              </span>
            </div>
          ))}
        </div>
      ) : null}

      {pdfUrl ? (
        <iframe
          className="editor-latex-frame"
          src={pdfUrl}
          title="LaTeX Preview"
          sandbox="allow-same-origin"
        />
      ) : (
        <div className="editor-latex-empty">
          {status === "compiling" ? "PDF olusuyor..." : "Onizleme yok"}
        </div>
      )}

      {/* Keep the log available for debugging without overwhelming the UI. */}
      {log ? (
        <details className="editor-latex-log">
          <summary>Derleme logu</summary>
          <pre>{log}</pre>
        </details>
      ) : null}
    </div>
  );
}
