import { arrayMove } from "@dnd-kit/sortable";

type ItemId = string | number;

export function reorderVisibleItems<TItem>({
  items,
  activeId,
  overId,
  getId,
  isVisible,
}: {
  items: TItem[];
  activeId: ItemId;
  overId: ItemId;
  getId: (item: TItem) => ItemId;
  isVisible: (item: TItem) => boolean;
}) {
  const visibleItems = items.filter(isVisible);
  const oldIndex = visibleItems.findIndex((item) => getId(item) === activeId);
  const newIndex = visibleItems.findIndex((item) => getId(item) === overId);

  if (oldIndex === -1 || newIndex === -1) return null;

  const reorderedVisibleItems = arrayMove(visibleItems, oldIndex, newIndex);
  let visibleIndex = 0;

  return items.map((item) => {
    if (!isVisible(item)) return item;
    return reorderedVisibleItems[visibleIndex++] ?? item;
  });
}
