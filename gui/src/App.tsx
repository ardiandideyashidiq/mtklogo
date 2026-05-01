import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { RepackPanel } from "@/components/repack-panel";
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

export default function App() {
  const [activePanel, setActivePanel] = useState<Panel>("unpack");
  const [binPath, setBinPath] = useState<string>("");
  const [screenResolution, setScreenResolution] = useState("1080x1920");
  const [lastUnpackedDir, setLastUnpackedDir] = useState<string>("");
  const [manualRepackDir, setManualRepackDir] = useState<string>("");
  const [stripAlpha, setStripAlpha] = useState(true);
  const [busy, setBusy] = useState<"unpack" | "repack" | null>(null);
  const [error, setError] = useState<string>("");
  const [notice, setNotice] = useState<NoticeState | null>(null);

  const currentRepackDir = manualRepackDir || lastUnpackedDir;

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
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  const repack = async () => {
    if (!currentRepackDir) {
      setError("Choose the extracted folder first.");
      return;
    }

    setBusy("repack");
    setError("");

    try {
      const result = await invoke<WorkflowResult>("repack_logo", {
        sourceDir: currentRepackDir,
        stripAlpha,
      });

      setNotice({
        title: "Repack complete",
        body: `Built ${result.file_count} files into:`,
        path: result.output_path,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <SidebarShell
      activePanel={activePanel}
      onSelectPanel={setActivePanel}
      busy={busy}
      binPath={binPath}
      unpackedDir={lastUnpackedDir}
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
          stripAlpha={stripAlpha}
          busy={busy}
          error={error}
          onChooseRepackDir={chooseRepackDir}
          onUseLatestUnpack={useLatestUnpack}
          onStripAlphaChange={setStripAlpha}
          onRepack={repack}
        />
      )}

      {notice ? <ResultDialog title={notice.title} message={notice.body} path={notice.path} onClose={() => setNotice(null)} /> : null}
    </SidebarShell>
  );
}
