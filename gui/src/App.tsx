import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";

type LogoSlot = {
  index: number;
  raw_size: number;
  inflated_size: number | null;
  inflation_error: string | null;
};

type LogoInspection = {
  slot_count: number;
  header_size: number;
  block_size: number;
  slots: LogoSlot[];
  supported_modes: string[];
};

type GuiFile = {
  name: string;
  bytes: number[];
};

const toBytes = async (file: File) => Array.from(new Uint8Array(await file.arrayBuffer()));

export default function App() {
  const [inspectFile, setInspectFile] = useState<File | null>(null);
  const [inspectResult, setInspectResult] = useState<LogoInspection | null>(null);
  const [inspectError, setInspectError] = useState<string | null>(null);
  const [inspectBusy, setInspectBusy] = useState(false);

  const [repackFiles, setRepackFiles] = useState<File[]>([]);
  const [stripAlpha, setStripAlpha] = useState(true);
  const [repackBusy, setRepackBusy] = useState(false);
  const [repackMessage, setRepackMessage] = useState<string | null>(null);

  const inspectLabel = useMemo(() => inspectFile?.name ?? "No file selected", [inspectFile]);
  const repackLabel = useMemo(() => (repackFiles.length ? `${repackFiles.length} file(s)` : "No files selected"), [repackFiles]);

  const analyze = async () => {
    if (!inspectFile) {
      setInspectError("Choose a logo.bin file first.");
      return;
    }

    setInspectBusy(true);
    setInspectError(null);
    setInspectResult(null);

    try {
      const bytes = await toBytes(inspectFile);
      const result = await invoke<LogoInspection>("inspect_logo", { bytes });
      setInspectResult(result);
    } catch (error) {
      setInspectError(error instanceof Error ? error.message : String(error));
    } finally {
      setInspectBusy(false);
    }
  };

  const repack = async () => {
    if (!repackFiles.length) {
      setRepackMessage("Choose at least one logo file first.");
      return;
    }

    setRepackBusy(true);
    setRepackMessage(null);

    try {
      const files: GuiFile[] = [];
      for (const file of repackFiles) {
        files.push({ name: file.name, bytes: await toBytes(file) });
      }

      const bytes = await invoke<number[]>("repack_logo", { files, stripAlpha });
      const blob = new Blob([new Uint8Array(bytes)], { type: "application/octet-stream" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = "logo.bin";
      link.click();
      URL.revokeObjectURL(url);
      setRepackMessage("Repacked logo.bin downloaded.");
    } catch (error) {
      setRepackMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setRepackBusy(false);
    }
  };

  return (
    <main className="mx-auto flex min-h-full w-full max-w-6xl flex-col gap-6 px-6 py-8">
      <section className="space-y-3">
        <Badge className="w-fit bg-primary/20 text-primary-foreground">CLI + GUI</Badge>
        <div className="space-y-2">
          <h1 className="text-4xl font-semibold tracking-tight">mtklogo GUI</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Inspect MTK logo blobs, preview slot sizes, and repack modified assets from the same core library that powers the CLI.
          </p>
        </div>
      </section>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Inspect logo.bin</CardTitle>
            <CardDescription>Load a binary image and see its slot layout at a glance.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Input type="file" accept=".bin,.img" onChange={(event) => setInspectFile(event.target.files?.[0] ?? null)} />
            <div className="flex items-center justify-between gap-3 text-sm text-muted-foreground">
              <span>{inspectLabel}</span>
              <Button onClick={analyze} disabled={inspectBusy}>
                {inspectBusy ? "Inspecting..." : "Inspect"}
              </Button>
            </div>
            {inspectError ? <p className="text-sm text-destructive">{inspectError}</p> : null}
            {inspectResult ? (
              <div className="space-y-4">
                <div className="grid gap-3 sm:grid-cols-3">
                  <Stat label="Slots" value={String(inspectResult.slot_count)} />
                  <Stat label="Header size" value={`${inspectResult.header_size} bytes`} />
                  <Stat label="Block size" value={`${inspectResult.block_size} bytes`} />
                </div>
                <div className="space-y-2">
                  <p className="text-sm font-medium">Slots</p>
                  <div className="space-y-2">
                    {inspectResult.slots.map((slot) => (
                      <div key={slot.index} className="rounded-lg border bg-card/60 px-3 py-2 text-sm">
                        <div className="flex items-center justify-between gap-3">
                          <span>Slot {slot.index}</span>
                          <span className="text-muted-foreground">{slot.raw_size} bytes</span>
                        </div>
                        <div className="mt-1 text-muted-foreground">
                          {slot.inflated_size !== null
                            ? `Inflated: ${slot.inflated_size} bytes`
                            : slot.inflation_error ?? "Not inflated"}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
                <div className="space-y-2">
                  <p className="text-sm font-medium">Supported modes</p>
                  <div className="flex flex-wrap gap-2">
                    {inspectResult.supported_modes.map((mode) => (
                      <Badge key={mode} variant="secondary">
                        {mode}
                      </Badge>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Repack logo assets</CardTitle>
            <CardDescription>Drop renamed slot files and download a rebuilt logo.bin.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Input
              type="file"
              multiple
              onChange={(event) => setRepackFiles(Array.from(event.target.files ?? []))}
            />
            <div className="flex items-center justify-between gap-3 text-sm text-muted-foreground">
              <span>{repackLabel}</span>
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={stripAlpha} onChange={(event) => setStripAlpha(event.target.checked)} />
                Strip alpha
              </label>
            </div>
            {repackFiles.length ? (
              <div className="max-h-48 space-y-2 overflow-auto rounded-lg border bg-card/60 p-3 text-sm">
                {repackFiles.map((file) => (
                  <div key={file.name} className="flex items-center justify-between gap-3">
                    <span className="truncate">{file.name}</span>
                    <span className="text-muted-foreground">{Math.round(file.size / 1024)} KB</span>
                  </div>
                ))}
              </div>
            ) : null}
            <Button onClick={repack} disabled={repackBusy}>
              {repackBusy ? "Repacking..." : "Repack"}
            </Button>
            {repackMessage ? <p className="text-sm text-muted-foreground">{repackMessage}</p> : null}
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border bg-card/60 p-3">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className="mt-1 text-lg font-semibold">{value}</p>
    </div>
  );
}
