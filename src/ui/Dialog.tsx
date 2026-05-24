import { Button } from "./Button";
import { cn } from "../utils/cn";
import {
  Dialog as DialogRoot,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/ui/shadcn/dialog";

export type DialogProps = {
  open: boolean;
  title: string;
  description?: string;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
  className?: string;
};

export function Dialog({
  open,
  title,
  description,
  onOpenChange,
  children,
  className,
}: DialogProps) {
  const contentProps = description ? {} : { "aria-describedby": undefined };

  return (
    <DialogRoot open={open} onOpenChange={onOpenChange}>
      <DialogContent className={cn(className)} {...contentProps}>
        <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3 sm:gap-4 sm:px-5 sm:py-4">
          <div className="min-w-0">
            <DialogTitle>{title}</DialogTitle>
            {description ? (
              <DialogDescription className="mt-1">{description}</DialogDescription>
            ) : null}
          </div>

          <DialogClose asChild>
            <Button variant="secondary" size="sm" aria-label="关闭" className="text-xs">
              关闭
            </Button>
          </DialogClose>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay px-4 py-3 sm:px-5 sm:py-4">
          {children}
        </div>
      </DialogContent>
    </DialogRoot>
  );
}
