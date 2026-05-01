import { cn } from "@/lib/utils";

type Panel = "unpack" | "repack";
type ThemeChoice = "light" | "dark" | "system";

type SidebarShellProps = {
  activePanel: Panel;
  onSelectPanel: (panel: Panel) => void;
  themeChoice: ThemeChoice;
  onThemeChange: (choice: ThemeChoice) => void;
  children: React.ReactNode;
};

const navItems: Array<{ id: Panel; title: string; subtitle: string }> = [
  { id: "unpack", title: "Unpack", subtitle: "Select .bin and extract" },
  { id: "repack", title: "Repack", subtitle: "Build .bin from folder" },
];

const themeItems: Array<{ id: ThemeChoice; label: string }> = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "System" },
];

export function SidebarShell({ activePanel, onSelectPanel, themeChoice, onThemeChange, children }: SidebarShellProps) {
  return (
    <main className="mx-auto flex h-full w-full max-w-7xl flex-col gap-4 p-3 sm:p-4 lg:flex-row lg:gap-5">
      <aside className="flex w-full flex-col rounded-lg border bg-card p-4 sm:p-5 lg:w-64 lg:shrink-0">
        <nav className="grid gap-2 md:grid-cols-2 lg:grid-cols-1">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => onSelectPanel(item.id)}
              className={cn(
                "w-full rounded-md border px-4 py-3 text-left leading-snug transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
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

        <div className="mt-5 space-y-3 border-t pt-4 lg:mt-auto">
          <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">Theme</p>
          <div className="grid grid-cols-3 gap-1.5 rounded-md border bg-background p-1.5">
            {themeItems.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => onThemeChange(item.id)}
                className={cn(
                  "rounded-sm px-2.5 py-2 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                  themeChoice === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                )}
                aria-pressed={themeChoice === item.id}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>

      </aside>

      <section className="min-w-0 flex-1 rounded-lg border bg-card p-5 sm:p-6">{children}</section>
    </main>
  );
}
