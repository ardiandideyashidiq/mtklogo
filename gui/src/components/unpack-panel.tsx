import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type UnpackPanelProps = {
  binPath: string;
  screenResolution: string;
  busy: "unpack" | "repack" | null;
  error: string;
  onSelectBin: () => Promise<void>;
  onScreenResolutionChange: (value: string) => void;
  onUnpack: () => Promise<void>;
};

export function UnpackPanel({
  binPath,
  screenResolution,
  busy,
  error,
  onSelectBin,
  onScreenResolutionChange,
  onUnpack,
}: UnpackPanelProps) {
  const canUnpack = binPath.length > 0 && screenResolution.length > 0;

  return (
    <div className="space-y-4">
      <header className="space-y-1">
        <h1 className="text-lg font-semibold">1. Unpack logo.bin</h1>
        <p className="text-sm text-muted-foreground">Select a source .bin file, set screen resolution, then extract.</p>
      </header>

      <div className="grid gap-3">
        <label className="space-y-1.5">
          <span className="text-sm font-medium">Source .bin file</span>
          <div className="flex gap-2">
            <Input value={binPath} readOnly placeholder="Select a logo.bin file" />
            <Button variant="outline" onClick={onSelectBin} disabled={busy !== null}>
              Select
            </Button>
          </div>
        </label>

        <label className="space-y-1.5">
          <span className="text-sm font-medium">Screen resolution</span>
          <Input
            value={screenResolution}
            onChange={(event) => onScreenResolutionChange(event.target.value)}
            placeholder="1080x1920"
            disabled={busy !== null}
          />
        </label>
      </div>

      <div className="flex min-h-9 items-center gap-3">
        <Button onClick={onUnpack} disabled={!canUnpack || busy !== null}>
          {busy === "unpack" ? "Unpacking..." : "Unpack"}
        </Button>
        <p className="text-xs text-muted-foreground">Creates a timestamped folder beside the input .bin file.</p>
      </div>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </div>
  );
}
