import { Button } from "@/components/ui/button";

type RepackPanelProps = {
  currentRepackDir: string;
  lastUnpackedDir: string;
  manualRepackDir: string;
  stripAlpha: boolean;
  busy: "unpack" | "repack" | null;
  error: string;
  onChooseRepackDir: () => Promise<void>;
  onUseLatestUnpack: () => void;
  onStripAlphaChange: (value: boolean) => void;
  onRepack: () => Promise<void>;
};

export function RepackPanel({
  currentRepackDir,
  lastUnpackedDir,
  manualRepackDir,
  stripAlpha,
  busy,
  error,
  onChooseRepackDir,
  onUseLatestUnpack,
  onStripAlphaChange,
  onRepack,
}: RepackPanelProps) {
  return (
    <div className="space-y-4">
      <header className="space-y-1">
        <h1 className="text-lg font-semibold">2. Repack folder to .bin</h1>
        <p className="text-sm text-muted-foreground">Uses latest unpacked folder by default. Override it by selecting another folder.</p>
      </header>

      <div className="space-y-1.5 rounded-md border bg-background p-3">
        <p className="text-sm font-medium">Source folder</p>
        <p className="break-all text-sm text-muted-foreground">{currentRepackDir || "No unpacked folder available yet"}</p>
        <p className="text-xs text-muted-foreground">Mode: {manualRepackDir ? "Manual selection" : "Latest unpack"}</p>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={onChooseRepackDir} disabled={busy !== null}>
          Choose folder
        </Button>
        <Button variant="outline" onClick={onUseLatestUnpack} disabled={!lastUnpackedDir || busy !== null}>
          Use latest unpack
        </Button>
      </div>

      <label className="flex min-h-9 items-center gap-2 text-sm">
        <input
          className="h-4 w-4 rounded border-input bg-background text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          type="checkbox"
          checked={stripAlpha}
          onChange={(event) => onStripAlphaChange(event.target.checked)}
        />
        Strip alpha before repacking
      </label>

      <div className="flex items-center gap-3">
        <Button onClick={onRepack} disabled={!currentRepackDir || busy !== null}>
          {busy === "repack" ? "Repacking..." : "Repack"}
        </Button>
        <p className="text-xs text-muted-foreground">Writes <code>.bin</code> next to the selected folder.</p>
      </div>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </div>
  );
}
