import type { ReactNode } from "react";
import { MainTopbar } from "../../app/components/MainTopbar";

type PhoneLayoutProps = {
  approvalToastsNode: ReactNode;
  updateToastNode: ReactNode;
  errorToastsNode: ReactNode;
  tabBarNode: ReactNode;
  sidebarNode: ReactNode;
  activeTab: "projects" | "codex" | "git" | "log" | "editor";
  activeWorkspace: boolean;
  showGitDetail: boolean;
  compactEmptyCodexNode: ReactNode;
  compactEmptyGitNode: ReactNode;
  compactGitBackNode: ReactNode;
  topbarLeftNode: ReactNode;
  messagesNode: ReactNode;
  editorNode: ReactNode;
  composerNode: ReactNode;
  gitDiffPanelNode: ReactNode;
  gitDiffViewerNode: ReactNode;
  debugPanelNode: ReactNode;
};

export function PhoneLayout({
  approvalToastsNode,
  updateToastNode,
  errorToastsNode,
  tabBarNode,
  sidebarNode,
  activeTab,
  activeWorkspace,
  showGitDetail,
  compactEmptyCodexNode,
  compactEmptyGitNode,
  compactGitBackNode,
  topbarLeftNode,
  messagesNode,
  editorNode,
  composerNode,
  gitDiffPanelNode,
  gitDiffViewerNode,
  debugPanelNode,
}: PhoneLayoutProps) {
  return (
    <div className="compact-shell">
      {approvalToastsNode}
      {updateToastNode}
      {errorToastsNode}
      {activeTab === "projects" && <div className="compact-panel">{sidebarNode}</div>}
      {activeTab === "codex" && (
        <div className="compact-panel">
          {activeWorkspace ? (
            <>
              <MainTopbar leftNode={topbarLeftNode} className="compact-topbar" />
              <div className="content compact-content">{messagesNode}</div>
              {composerNode}
            </>
          ) : (
            compactEmptyCodexNode
          )}
        </div>
      )}
      {activeTab === "git" && (
        <div className="compact-panel">
          {!activeWorkspace && compactEmptyGitNode}
          {activeWorkspace && showGitDetail && (
            <>
              {compactGitBackNode}
              <div className="compact-git-viewer">{gitDiffViewerNode}</div>
            </>
          )}
          {activeWorkspace && !showGitDetail && (
            <>
              <MainTopbar leftNode={topbarLeftNode} className="compact-topbar" />
              <div className="compact-git">
                <div className="compact-git-list">{gitDiffPanelNode}</div>
              </div>
            </>
          )}
        </div>
      )}
      {activeTab === "log" && (
        <div className="compact-panel">{debugPanelNode}</div>
      )}
      {activeTab === "editor" && (
        <div className="compact-panel">
          <MainTopbar leftNode={topbarLeftNode} className="compact-topbar" />
          <div className="content compact-content">{editorNode}</div>
        </div>
      )}
      {tabBarNode}
    </div>
  );
}
