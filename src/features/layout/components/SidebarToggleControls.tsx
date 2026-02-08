import PanelLeftClose from "lucide-react/dist/esm/icons/panel-left-close";
import PanelLeftOpen from "lucide-react/dist/esm/icons/panel-left-open";
import PanelRightClose from "lucide-react/dist/esm/icons/panel-right-close";
import PanelRightOpen from "lucide-react/dist/esm/icons/panel-right-open";

export type SidebarToggleProps = {
  isCompact: boolean;
  sidebarCollapsed: boolean;
  rightPanelCollapsed: boolean;
  onCollapseSidebar: () => void;
  onExpandSidebar: () => void;
  onCollapseRightPanel: () => void;
  onExpandRightPanel: () => void;
};

export function SidebarCollapseButton({
  isCompact,
  sidebarCollapsed,
  onExpandSidebar,
  onCollapseSidebar,
}: SidebarToggleProps) {
  if (isCompact) {
    return null;
  }
  return (
    <button
      type="button"
      className="ghost main-header-action"
      onClick={sidebarCollapsed ? onExpandSidebar : onCollapseSidebar}
      data-tauri-drag-region="false"
      aria-label={sidebarCollapsed ? "Show threads sidebar" : "Hide threads sidebar"}
      title={sidebarCollapsed ? "Show threads sidebar" : "Hide threads sidebar"}
    >
      {sidebarCollapsed ? (
        <PanelLeftOpen size={14} aria-hidden />
      ) : (
        <PanelLeftClose size={14} aria-hidden />
      )}
    </button>
  );
}

export function RightPanelCollapseButton({
  isCompact,
  rightPanelCollapsed,
  onExpandRightPanel,
  onCollapseRightPanel,
}: SidebarToggleProps) {
  if (isCompact) {
    return null;
  }
  return (
    <button
      type="button"
      className="ghost main-header-action"
      onClick={rightPanelCollapsed ? onExpandRightPanel : onCollapseRightPanel}
      data-tauri-drag-region="false"
      aria-label={rightPanelCollapsed ? "Show git sidebar" : "Hide git sidebar"}
      title={rightPanelCollapsed ? "Show git sidebar" : "Hide git sidebar"}
    >
      {rightPanelCollapsed ? (
        <PanelRightOpen size={14} aria-hidden />
      ) : (
        <PanelRightClose size={14} aria-hidden />
      )}
    </button>
  );
}

export function TitlebarExpandControls({
  isCompact,
  sidebarCollapsed,
  rightPanelCollapsed,
  onExpandSidebar,
  onExpandRightPanel,
}: SidebarToggleProps) {
  if (isCompact || (!sidebarCollapsed && !rightPanelCollapsed)) {
    return null;
  }
  return (
    <div className="titlebar-controls">
      {sidebarCollapsed && (
        <div className="titlebar-toggle titlebar-toggle-left">
          <button
            type="button"
            className="ghost main-header-action"
            onClick={onExpandSidebar}
            data-tauri-drag-region="false"
            aria-label="Show threads sidebar"
            title="Show threads sidebar"
          >
            <PanelLeftOpen size={14} aria-hidden />
          </button>
        </div>
      )}
      {rightPanelCollapsed && (
        <div className="titlebar-toggle titlebar-toggle-right">
          <button
            type="button"
            className="ghost main-header-action"
            onClick={onExpandRightPanel}
            data-tauri-drag-region="false"
            aria-label="Show git sidebar"
            title="Show git sidebar"
          >
            <PanelRightOpen size={14} aria-hidden />
          </button>
        </div>
      )}
    </div>
  );
}
