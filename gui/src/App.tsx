import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowRight, FolderSearch, FolderUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ProgressDialog } from "@/components/progress-dialog";
import { ResultDialog } from "@/components/result-dialog";

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
  const [binPath, setBinPath] = useState<string>("");
  const [screenResolution, setScreenResolution] = useState("1080x1920");
  const [lastUnpackedDir, setLastUnpackedDir] = useState<string>("");
  const [manualRepackDir, setManualRepackDir] = useState<string>("");
  const [busy, setBusy] = useState<"unpack" | "repack" | null>(null);
  const [workflow, setWorkflow] = useState<WorkflowProgress | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [error, setError] = useState<string>("");
  const [notice, setNotice] = useState<NoticeState | null>(null);

  const currentRepackDir = manualRepackDir || lastUnpackedDir;

  useEffect(() => {
    const root = document.documentElement;

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      const resolved = media.matches ? "dark" : "light";
      root.dataset.theme = resolved;
      root.style.colorScheme = resolved;
    };

    applyTheme();

    const onChange = () => applyTheme();
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

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
    }
  };

  const chooseRepackDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setManualRepackDir(selected);
      setError("");
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
    <main className="relative mx-auto grid min-h-full w-full max-w-7xl grid-cols-1 gap-4 px-3 py-3 sm:px-4 sm:py-4 lg:grid-cols-2 lg:gap-6 lg:px-6 lg:py-6">
      <section className="rounded-xl border bg-card p-4 shadow-sm sm:p-5">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">Step 1</p>
            <h2 className="mt-1 text-sm font-semibold">Choose source and resolution</h2>
          </div>
          <span className="text-xs text-muted-foreground">Before unpack</span>
        </div>

        <div className="mt-4 grid gap-4">
          <label className="space-y-1.5">
            <span className="text-sm font-medium">Source .bin file</span>
            <div className="flex gap-2">
              <Input value={binPath} readOnly placeholder="Select a logo.bin file" className="h-11" />
              <Button variant="outline" onClick={chooseBin} disabled={busy !== null} className="h-11 shrink-0 px-4">
                <FolderUp className="mr-2 h-4 w-4" aria-hidden="true" />
                Select
              </Button>
            </div>
          </label>

          <label className="space-y-1.5">
            <span className="text-sm font-medium">Screen resolution</span>
            <Input
              value={screenResolution}
              onChange={(event) => setScreenResolution(event.target.value)}
              placeholder="1080x1920"
              disabled={busy !== null}
              className="h-11"
            />
          </label>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-3">
          <Button onClick={unpack} disabled={!binPath || !screenResolution || busy !== null} className="h-11 px-5">
            {busy === "unpack" ? "Unpacking..." : "Unpack"}
          </Button>
        </div>

        {error ? <p className="mt-4 px-1 text-sm text-destructive">{error}</p> : null}
      </section>

      <section className="rounded-xl border bg-card p-4 shadow-sm sm:p-5">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">Step 2</p>
            <h2 className="mt-1 text-sm font-semibold">Repack the extracted folder</h2>
          </div>
        </div>

        <div className="mt-4 space-y-3">
          <div className="rounded-lg border bg-background p-3.5">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Current folder</p>
                <p className="mt-1 break-all text-sm text-foreground">{currentRepackDir || "Use the unpack output or choose another folder"}</p>
              </div>
              {lastUnpackedDir ? <FolderSearch className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" /> : null}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2.5">
            <Button variant="outline" onClick={useLatestUnpack} disabled={!lastUnpackedDir || busy !== null} className="h-11 px-4">
              Use unpack output
            </Button>
            <Button variant="outline" onClick={chooseRepackDir} disabled={busy !== null} className="h-11 px-4">
              Choose folder
            </Button>
            <Button onClick={repack} disabled={!currentRepackDir || busy !== null} className="h-11 px-5">
              {busy === "repack" ? "Repacking..." : "Repack"}
              <ArrowRight className="ml-2 h-4 w-4" aria-hidden="true" />
            </Button>
          </div>
        </div>

        {error ? <p className="mt-4 px-1 text-sm text-destructive">{error}</p> : null}
      </section>

      {notice ? <ResultDialog title={notice.title} message={notice.body} path={notice.path} onClose={() => setNotice(null)} /> : null}
      {workflow ? <ProgressDialog progress={workflow} cancelling={cancelling} onCancel={cancelWorkflow} /> : null}
    </main>
  );
}
