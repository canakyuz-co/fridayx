type EditorPlaceholderProps = {
  hasWorkspace: boolean;
};

export function EditorPlaceholder({ hasWorkspace }: EditorPlaceholderProps) {
  return (
    <div className="editor-shell">
      <div className="editor-placeholder">
        <h2 className="editor-placeholder-title">Editor hazirlanıyor</h2>
        <p className="editor-placeholder-text">
          {hasWorkspace
            ? "Dosya agaci ve editor paneli bir sonraki adimda acilacak."
            : "Once bir workspace secerek dosya listesini yukleyelim."}
        </p>
      </div>
    </div>
  );
}
