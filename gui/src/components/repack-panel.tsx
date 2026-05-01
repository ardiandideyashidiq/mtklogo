import { Button } from "@/components/ui/button";

type RepackPanelProps = {
  currentRepackDir: string;
  lastUnpackedDir: string;
  manualRepackDir: string;
  busy: "unpack" | "repack" | null;
  error: string;
  onChooseRepackDir: () => Promise<void>;
  onUseLatestUnpack: () => void;
  onRepack: () => Promise<void>;
};

export function RepackPanel({
  currentRepackDir,
  lastUnpackedDir,
  manualRepackDir,
  busy,
  error,
  onChooseRepackDir,
  onUseLatestUnpack,
  onRepack,
}: RepackPanelProps) {
  return (
    <div className="space-y-6">
      <div className="space-y-2 rounded-md border bg-background p-4">
        <p className="text-sm font-medium">Source folder</p>
        <p className="break-all text-sm text-muted-foreground">{currentRepackDir || "No unpacked folder available yet"}</p>
        <p className="text-xs text-muted-foreground">Mode: {manualRepackDir ? "Manual selection" : "Latest unpack"}</p>
      </div>

      <div className="flex flex-wrap gap-2.5">
        <Button variant="outline" onClick={onChooseRepackDir} disabled={busy !== null}>
          Choose folder
        </Button>
        <Button variant="outline" onClick={onUseLatestUnpack} disabled={!lastUnpackedDir || busy !== null}>
          Use latest unpack
        </Button>
      </div>

      <div className="flex items-center gap-3 pt-1">
        <Button onClick={onRepack} disabled={!currentRepackDir || busy !== null}>
          {busy === "repack" ? "Repacking..." : "Repack"}
        </Button>
      </div>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </div>
  );
}
