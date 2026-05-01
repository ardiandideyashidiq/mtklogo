import { Button } from "@/components/ui/button";

type ResultDialogProps = {
  title: string;
  message: string;
  path: string;
  onClose: () => void;
};

export function ResultDialog({ title, message, path, onClose }: ResultDialogProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/75 p-4 backdrop-blur-sm">
      <div role="dialog" aria-modal="true" className="w-full max-w-xl rounded-lg border bg-card p-5 shadow-lg">
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="mt-1.5 text-sm text-muted-foreground">{message}</p>
        <p className="mt-3 break-all rounded-md border bg-background px-3 py-2 text-sm">{path}</p>
        <div className="mt-5 flex justify-end">
          <Button onClick={onClose}>Close</Button>
        </div>
      </div>
    </div>
  );
}
