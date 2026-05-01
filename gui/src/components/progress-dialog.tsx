import { Button } from "@/components/ui/button";

type WorkflowKind = "unpack" | "repack";

type WorkflowProgress = {
  workflow: WorkflowKind;
  phase: string;
  current: number;
  total: number;
  message: string;
  item: string | null;
};

type ProgressDialogProps = {
  progress: WorkflowProgress;
  cancelling: boolean;
  onCancel: () => Promise<void>;
};

export function ProgressDialog({ progress, cancelling, onCancel }: ProgressDialogProps) {
  const percent = progress.total > 0 ? Math.min(100, Math.round((progress.current / progress.total) * 100)) : 0;
  const title = progress.workflow === "unpack" ? "Unpacking" : "Repacking";
  const scale = percent / 100;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/75 p-4 backdrop-blur-sm">
      <div role="dialog" aria-modal="true" className="w-full max-w-xl rounded-lg border bg-card p-6 shadow-lg">
        <div className="flex items-start justify-between gap-5">
          <div>
            <h2 className="text-lg font-semibold">{title}</h2>
            <p className="mt-1.5 text-sm text-muted-foreground">{progress.message}</p>
          </div>
          <div className="rounded-full border bg-background px-3 py-1 text-xs font-medium text-muted-foreground">
            {percent}%
          </div>
        </div>

        <div className="mt-5 h-2 overflow-hidden rounded-full bg-muted">
          <div
            className="h-full w-full origin-left rounded-full bg-primary transition-transform duration-200 ease-out"
            style={{ transform: `scaleX(${scale})` }}
          />
        </div>

        <div className="mt-4 flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {progress.current} / {progress.total}
          </span>
          <span className="capitalize">{progress.phase}</span>
        </div>

        {progress.item ? <p className="mt-4 break-all text-sm text-foreground">{progress.item}</p> : null}

        <div className="mt-6 flex justify-end">
          <Button variant="outline" onClick={onCancel} disabled={cancelling}>
            {cancelling ? "Cancelling..." : "Cancel"}
          </Button>
        </div>
      </div>
    </div>
  );
}
