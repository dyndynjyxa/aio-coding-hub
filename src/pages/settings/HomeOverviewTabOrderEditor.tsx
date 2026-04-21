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
import { useMemo, useState } from "react";
import {
  HOME_OVERVIEW_TABS,
  readHomeOverviewTabOrderFromStorage,
  writeHomeOverviewTabOrderToStorage,
  type HomeOverviewTabKey,
} from "../../services/home/homeOverviewTabOrder";
import { Button } from "../../ui/Button";
import { cn } from "../../utils/cn";

type SortableOrderButtonProps = {
  item: { key: HomeOverviewTabKey; label: string };
};

function SortableOrderButton({ item }: SortableOrderButtonProps) {
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

export function HomeOverviewTabOrderEditor() {
  const [order, setOrder] = useState<HomeOverviewTabKey[]>(() =>
    readHomeOverviewTabOrderFromStorage()
  );
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  const orderedTabs = useMemo(() => {
    const labelByKey = new Map(HOME_OVERVIEW_TABS.map((item) => [item.key, item.label]));
    return order.map((key) => ({ key, label: labelByKey.get(key) ?? key }));
  }, [order]);

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    setOrder((currentOrder) => {
      const fromIndex = currentOrder.indexOf(active.id as HomeOverviewTabKey);
      const toIndex = currentOrder.indexOf(over.id as HomeOverviewTabKey);
      if (fromIndex < 0 || toIndex < 0) return currentOrder;

      const nextOrder = arrayMove(currentOrder, fromIndex, toIndex);
      writeHomeOverviewTabOrderToStorage(nextOrder);
      return nextOrder;
    });
  }

  return (
    <div className="inline-block max-w-full rounded-lg border border-slate-200 bg-slate-50/70 p-1.5 dark:border-slate-700 dark:bg-slate-800/40">
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext
          items={orderedTabs.map((item) => item.key)}
          strategy={horizontalListSortingStrategy}
        >
          <div className="flex flex-wrap gap-1">
            {orderedTabs.map((item) => (
              <SortableOrderButton key={item.key} item={item} />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
