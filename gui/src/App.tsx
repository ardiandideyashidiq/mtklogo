import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";

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

export default function App() {
  const [binPath, setBinPath] = useState<string>("");
  const [screenResolution, setScreenResolution] = useState("1080x1920");
  const [lastUnpackedDir, setLastUnpackedDir] = useState<string>("");
  const [manualRepackDir, setManualRepackDir] = useState<string>("");
  const [stripAlpha, setStripAlpha] = useState(true);
  const [busy, setBusy] = useState<"unpack" | "repack" | null>(null);
  const [error, setError] = useState<string>("");
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const currentRepackDir = manualRepackDir || lastUnpackedDir;

  const canUnpack = binPath.length > 0 && screenResolution.length > 0;
  const canRepack = currentRepackDir.length > 0;

  const statusLabel = useMemo(() => {
    if (busy === "unpack") return "Unpacking...";
    if (busy === "repack") return "Repacking...";
    return "Ready";
  }, [busy]);

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

  const resetSession = () => {
    setBinPath("");
    setScreenResolution("1080x1920");
    setLastUnpackedDir("");
    setManualRepackDir("");
    setStripAlpha(true);
    setBusy(null);
    setError("");
    setNotice(null);
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
        body: `Extracted ${result.file_count} files to`,
        path: result.output_path,
      });
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
        body: `Built ${result.file_count} files into`,
        path: result.output_path,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <main className="mx-auto flex min-h-full w-full max-w-5xl flex-col gap-6 px-6 py-8">
      <section className="space-y-3">
        <Badge className="w-fit bg-primary/20 text-primary-foreground">CLI + GUI</Badge>
        <div className="space-y-2">
          <h1 className="text-4xl font-semibold tracking-tight">mtklogo GUI</h1>
          <p className="max-w-3xl text-sm text-muted-foreground">
            Pick a `.bin`, unpack it into a timestamped folder beside the source file, edit the images manually, then repack from the latest folder or a folder you choose.
          </p>
        </div>
      </section>

      <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader>
            <CardTitle>Workflow</CardTitle>
            <CardDescription>Unpack first, edit files in the extracted directory, then repack from that directory.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <div className="grid gap-3">
              <label className="space-y-2">
                <span className="text-sm font-medium">Selected .bin</span>
                <div className="flex gap-2">
                  <Input value={binPath} readOnly placeholder="Choose a logo.bin file" />
                  <Button variant="outline" onClick={chooseBin}>
                    Select
                  </Button>
                </div>
              </label>

              <label className="space-y-2">
                <span className="text-sm font-medium">Screen resolution</span>
                <Input
                  value={screenResolution}
                  onChange={(event) => setScreenResolution(event.target.value)}
                  placeholder="1080x1920"
                />
              </label>

              <div className="flex flex-wrap gap-3">
                <Button onClick={unpack} disabled={!canUnpack || busy !== null}>
                  {busy === "unpack" ? "Unpacking..." : "Unpack"}
                </Button>
                <Button variant="outline" onClick={resetSession}>
                  New session
                </Button>
              </div>
            </div>

            <div className="rounded-xl border bg-card/50 p-4 text-sm">
              <p className="font-medium">Current state</p>
              <div className="mt-2 space-y-1 text-muted-foreground">
                <p>Input: {binPath || "none"}</p>
                <p>Resolution: {screenResolution || "none"}</p>
                <p>Latest unpacked folder: {lastUnpackedDir || "none"}</p>
                <p>Repack folder: {currentRepackDir || "none"}</p>
              </div>
            </div>

            {error ? <p className="text-sm text-destructive">{error}</p> : null}
            <p className="text-sm text-muted-foreground">Status: {statusLabel}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Repack</CardTitle>
            <CardDescription>Use the most recent unpacked folder by default, or choose another folder manually.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2 rounded-xl border bg-card/50 p-4 text-sm">
              <p className="font-medium">Source folder</p>
              <p className="break-all text-muted-foreground">{currentRepackDir || "No extracted folder yet"}</p>
            </div>

            <div className="flex flex-wrap gap-3">
              <Button variant="outline" onClick={chooseRepackDir}>
                Choose folder
              </Button>
              <Button variant="outline" onClick={useLatestUnpack} disabled={!lastUnpackedDir}>
                Use latest unpack
              </Button>
            </div>

            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={stripAlpha} onChange={(event) => setStripAlpha(event.target.checked)} />
              Strip alpha before repacking
            </label>

            <Button onClick={repack} disabled={!canRepack || busy !== null}>
              {busy === "repack" ? "Repacking..." : "Repack"}
            </Button>

            <p className="text-sm text-muted-foreground">
              The rebuilt `.bin` is written beside the source folder using the same folder name plus `.bin`.
            </p>
          </CardContent>
        </Card>
      </div>

      {notice ? <NoticeModal notice={notice} onClose={() => setNotice(null)} /> : null}
    </main>
  );
}

function NoticeModal({ notice, onClose }: { notice: NoticeState; onClose: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-6">
      <div className="w-full max-w-lg rounded-2xl border bg-card p-6 shadow-2xl">
        <h2 className="text-xl font-semibold">{notice.title}</h2>
        <p className="mt-2 text-sm text-muted-foreground">{notice.body}</p>
        <p className="mt-3 break-all rounded-xl border bg-background px-3 py-2 text-sm">{notice.path}</p>
        <div className="mt-6 flex justify-end">
          <Button onClick={onClose}>OK</Button>
        </div>
      </div>
    </div>
  );
}
