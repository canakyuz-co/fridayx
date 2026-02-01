import { useEffect } from "react";
import type { AppSettings } from "../../../types";
import { buildInterFontFeatureSettings, isInterFontFamily } from "../../../utils/fonts";

export function useCodeCssVars(appSettings: AppSettings) {
  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    const root = document.documentElement;
    root.style.setProperty("--code-font-family", appSettings.codeFontFamily);
    root.style.setProperty("--code-font-size", `${appSettings.codeFontSize}px`);
    const uiFeatures = isInterFontFamily(appSettings.uiFontFamily)
      ? buildInterFontFeatureSettings(appSettings.interFontFeatures)
      : "normal";
    root.style.setProperty("--ui-font-features", uiFeatures);
  }, [
    appSettings.codeFontFamily,
    appSettings.codeFontSize,
    appSettings.interFontFeatures,
    appSettings.uiFontFamily,
  ]);
}
