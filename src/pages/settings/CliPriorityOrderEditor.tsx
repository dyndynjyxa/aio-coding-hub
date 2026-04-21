import {
  DndContext,
  PointerSensor,
  closestCenter,
  type DragEndEvent,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  horizontalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import { useMemo } from "react";
import { getOrderedClis } from "../../services/cli/cliPriorityOrder";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { cn } from "../../utils/cn";

type SortableCliButtonProps = {
  item: { key: CliKey; label: string };
};

function SortableCliButton({ item }: SortableCliButtonProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: item.key,
  });
  const { role: _ignoredRole, ...sortableAttributes } = attributes;

  return (
    <Button
      ref={setNodeRef}
      variant="secondary"
      size="sm"
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
      className={cn(
        "h-auto shrink-0 justify-start gap-1 px-2 py-1 text-left shadow-none touch-none select-none",
        "cursor-grab active:cursor-grabbing",
        isDragging && "z-10 opacity-60"
      )}
      {...sortableAttributes}
      {...listeners}
    >
      <GripVertical className="h-3 w-3 shrink-0 opacity-45" aria-hidden="true" />
      <span className="text-[11px] font-medium whitespace-nowrap">{item.label}</span>
    </Button>
  );
}

export function CliPriorityOrderEditor({
  order,
  onChange,
}: {
  order: CliKey[];
  onChange: (nextOrder: CliKey[]) => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );
  const orderedClis = useMemo(
    () => getOrderedClis(order).map((cli) => ({ key: cli.key, label: cli.name })),
    [order]
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const currentOrder = orderedClis.map((item) => item.key);
    const fromIndex = currentOrder.indexOf(active.id as CliKey);
    const toIndex = currentOrder.indexOf(over.id as CliKey);
    if (fromIndex < 0 || toIndex < 0) return;
    onChange(arrayMove(currentOrder, fromIndex, toIndex));
  }

  return (
    <div className="inline-block max-w-full rounded-lg border border-slate-200 bg-slate-50/70 p-1.5 dark:border-slate-700 dark:bg-slate-800/40">
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext
          items={orderedClis.map((item) => item.key)}
          strategy={horizontalListSortingStrategy}
        >
          <div className="flex gap-1 overflow-x-auto pb-0.5">
            {orderedClis.map((item) => (
              <SortableCliButton key={item.key} item={item} />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
