import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type Panel = "unpack" | "repack";

type SidebarShellProps = {
  activePanel: Panel;
  onSelectPanel: (panel: Panel) => void;
  busy: "unpack" | "repack" | null;
  binPath: string;
  unpackedDir: string;
  children: React.ReactNode;
};

const navItems: Array<{ id: Panel; title: string; subtitle: string }> = [
  { id: "unpack", title: "Unpack", subtitle: "Select .bin and extract" },
  { id: "repack", title: "Repack", subtitle: "Build .bin from folder" },
];

export function SidebarShell({ activePanel, onSelectPanel, busy, binPath, unpackedDir, children }: SidebarShellProps) {
  const status = busy ? `${busy === "unpack" ? "Unpacking" : "Repacking"}...` : "Ready";

  return (
    <main className="mx-auto flex h-full w-full max-w-6xl gap-3 p-3">
      <aside className="flex w-64 shrink-0 flex-col rounded-lg border bg-card p-3">
        <div className="space-y-1 border-b pb-3">
          <p className="text-sm font-semibold">mtklogo utility</p>
          <p className="text-xs text-muted-foreground">Desktop workflow for MTK logo files</p>
        </div>

        <nav className="mt-3 space-y-1.5">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => onSelectPanel(item.id)}
              className={cn(
                "w-full rounded-md border px-3 py-2 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                activePanel === item.id
                  ? "border-primary/50 bg-primary/10"
                  : "border-border bg-background hover:bg-muted/40",
              )}
            >
              <p className="text-sm font-medium">{item.title}</p>
              <p className="text-[11px] text-muted-foreground">{item.subtitle}</p>
            </button>
          ))}
        </nav>

        <div className="mt-auto space-y-2 rounded-md border bg-background p-3 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Status</span>
            <Badge>{status}</Badge>
          </div>
          <p className="truncate text-muted-foreground">Input: {binPath || "none"}</p>
          <p className="truncate text-muted-foreground">Latest unpack: {unpackedDir || "none"}</p>
        </div>
      </aside>

      <section className="min-w-0 flex-1 rounded-lg border bg-card p-4">{children}</section>
    </main>
  );
}
