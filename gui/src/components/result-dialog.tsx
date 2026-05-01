import { useState } from "react";
import { Button } from "@/components/ui/button";

type ResultDialogProps = {
  title: string;
  message: string;
  path: string;
  onClose: () => void;
};

export function ResultDialog({ title, message, path, onClose }: ResultDialogProps) {
  const [copyLabel, setCopyLabel] = useState("Copy path");

  const copyPath = async () => {
    try {
      await navigator.clipboard.writeText(path);
      setCopyLabel("Copied");
      window.setTimeout(() => setCopyLabel("Copy path"), 1200);
    } catch {
      setCopyLabel("Copy failed");
      window.setTimeout(() => setCopyLabel("Copy path"), 1200);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/75 p-4 backdrop-blur-sm">
      <div role="dialog" aria-modal="true" className="w-full max-w-xl rounded-lg border bg-card p-6 shadow-lg">
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="mt-2 text-sm text-muted-foreground">{message}</p>
        <p className="mt-4 break-all rounded-md border bg-background px-3 py-2.5 text-sm">{path}</p>
        <div className="mt-6 flex justify-end gap-2.5">
          <Button variant="outline" onClick={copyPath}>
            {copyLabel}
          </Button>
          <Button onClick={onClose}>Close</Button>
        </div>
      </div>
    </div>
  );
}
