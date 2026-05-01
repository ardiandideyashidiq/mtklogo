import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RepackPanel } from "@/components/repack-panel";
import { ProgressDialog } from "@/components/progress-dialog";
import { ResultDialog } from "@/components/result-dialog";
import { SidebarShell } from "@/components/sidebar-shell";
import { UnpackPanel } from "@/components/unpack-panel";

type WorkflowResult = {
  output_path: string;
  timestamp: string;
  file_count: number;
};

type NoticeState = {
  title: string;
  body: string;
  path: string;
};

type Panel = "unpack" | "repack";
type ThemeChoice = "light" | "dark" | "system";
type WorkflowKind = "unpack" | "repack";

type WorkflowProgress = {
  workflow: WorkflowKind;
  phase: string;
  current: number;
  total: number;
  message: string;
  item: string | null;
};

export default function App() {
  const [activePanel, setActivePanel] = useState<Panel>("unpack");
  const [binPath, setBinPath] = useState<string>("");
  const [screenResolution, setScreenResolution] = useState("1080x1920");
  const [lastUnpackedDir, setLastUnpackedDir] = useState<string>("");
  const [manualRepackDir, setManualRepackDir] = useState<string>("");
  const [busy, setBusy] = useState<"unpack" | "repack" | null>(null);
  const [workflow, setWorkflow] = useState<WorkflowProgress | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [error, setError] = useState<string>("");
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const [themeChoice, setThemeChoice] = useState<ThemeChoice>(() => {
    const saved = window.localStorage.getItem("mtklogo.theme");
    return saved === "light" || saved === "dark" || saved === "system" ? saved : "system";
  });

  const currentRepackDir = manualRepackDir || lastUnpackedDir;

  useEffect(() => {
    const root = document.documentElement;
    const storageKey = "mtklogo.theme";

    const applyTheme = () => {
      const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      const resolved = themeChoice === "dark" || (themeChoice === "system" && systemDark) ? "dark" : "light";
      root.dataset.theme = resolved;
      root.dataset.themeChoice = themeChoice;
      root.style.colorScheme = resolved;
      window.localStorage.setItem(storageKey, themeChoice);
    };

    applyTheme();

    if (themeChoice !== "system") {
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme();
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [themeChoice]);

  useEffect(() => {
    let unlisten: (() => Promise<void>) | undefined;

    listen<WorkflowProgress>("workflow-progress", (event) => {
      setWorkflow(event.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const chooseBin = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "MTK logo", extensions: ["bin", "img"] }],
    });

    if (typeof selected === "string") {
      setBinPath(selected);
      setLastUnpackedDir("");
      setManualRepackDir("");
      setNotice(null);
      setError("");
      setActivePanel("unpack");
    }
  };

  const chooseRepackDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setManualRepackDir(selected);
      setError("");
      setActivePanel("repack");
    }
  };

  const useLatestUnpack = () => {
    setManualRepackDir("");
  };

  const unpack = async () => {
    if (!binPath) {
      setError("Select a .bin file first.");
      return;
    }
    if (!screenResolution) {
      setError("Enter a screen resolution first.");
      return;
    }

    setBusy("unpack");
    setError("");
    setCancelling(false);
    setWorkflow({
      workflow: "unpack",
      phase: "starting",
      current: 0,
      total: 1,
      message: "Starting unpack",
      item: null,
    });

    try {
      const result = await invoke<WorkflowResult>("unpack_logo", {
        inputPath: binPath,
        screenResolution,
      });

      setLastUnpackedDir(result.output_path);
      setManualRepackDir("");
      setNotice({
        title: "Extraction complete",
        body: `Extracted ${result.file_count} files to:`,
        path: result.output_path,
      });
      setActivePanel("repack");
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      if (message !== "operation cancelled") {
        setError(message);
      }
    } finally {
      setBusy(null);
      setCancelling(false);
      setWorkflow(null);
    }
  };

  const repack = async () => {
    if (!currentRepackDir) {
      setError("Choose the extracted folder first.");
      return;
    }

    setBusy("repack");
    setError("");
    setCancelling(false);
    setWorkflow({
      workflow: "repack",
      phase: "starting",
      current: 0,
      total: 1,
      message: "Starting repack",
      item: null,
    });

    try {
      const result = await invoke<WorkflowResult>("repack_logo", {
        sourceDir: currentRepackDir,
        stripAlpha: true,
      });

      setNotice({
        title: "Repack complete",
        body: `Built ${result.file_count} files into:`,
        path: result.output_path,
      });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      if (message !== "operation cancelled") {
        setError(message);
      }
    } finally {
      setBusy(null);
      setCancelling(false);
      setWorkflow(null);
    }
  };

  const cancelWorkflow = async () => {
    if (!workflow || cancelling) {
      return;
    }

    setCancelling(true);
    try {
      await invoke("cancel_current_workflow");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setCancelling(false);
    }
  };

  return (
    <SidebarShell
      activePanel={activePanel}
      onSelectPanel={setActivePanel}
      themeChoice={themeChoice}
      onThemeChange={setThemeChoice}
    >
      {activePanel === "unpack" ? (
        <UnpackPanel
          binPath={binPath}
          screenResolution={screenResolution}
          busy={busy}
          error={error}
          onSelectBin={chooseBin}
          onScreenResolutionChange={setScreenResolution}
          onUnpack={unpack}
        />
      ) : (
        <RepackPanel
          currentRepackDir={currentRepackDir}
          lastUnpackedDir={lastUnpackedDir}
          manualRepackDir={manualRepackDir}
          busy={busy}
          error={error}
          onChooseRepackDir={chooseRepackDir}
          onUseLatestUnpack={useLatestUnpack}
          onRepack={repack}
        />
      )}

      {notice ? <ResultDialog title={notice.title} message={notice.body} path={notice.path} onClose={() => setNotice(null)} /> : null}
      {workflow ? <ProgressDialog progress={workflow} cancelling={cancelling} onCancel={cancelWorkflow} /> : null}
    </SidebarShell>
  );
}
