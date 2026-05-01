import * as React from "react";
import { cn } from "@/lib/utils";

export function Badge({
  className,
  variant = "default",
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { variant?: "default" | "secondary" }) {
  const variants = {
    default: "border border-primary/35 bg-primary/15 text-primary-foreground",
    secondary: "border border-border bg-secondary text-secondary-foreground",
  };

  return (
    <span
        className={cn(
          "inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium tracking-wide",
          variants[variant],
          className,
        )}
      {...props}
    />
  );
}
