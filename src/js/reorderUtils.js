export function clamp(value, min, max) {
    return Math.max(min, Math.min(value, max));
}

export function reorderArray(items, fromIndex, toIndex) {
    const copy = [...items];
    if (copy.length === 0) return copy;

    const from = clamp(fromIndex, 0, copy.length - 1);
    const to = clamp(toIndex, 0, copy.length - 1);
    const [moved] = copy.splice(from, 1);
    copy.splice(to, 0, moved);
    return copy;
}

export function updateDragVisualNumbersByDom(listEl, sourceIndex, targetIndex, draggedEl) {
    if (!listEl) return;

    const items = Array.from(listEl.querySelectorAll('.mod-item'));
    if (items.length === 0) return;

    const boundedSource = clamp(sourceIndex ?? 0, 0, items.length - 1);
    const boundedTarget = clamp(targetIndex ?? boundedSource, 0, items.length - 1);
    const dragged = draggedEl && items.includes(draggedEl) ? draggedEl : items[boundedSource];
    if (!dragged) return;

    const reordered = [...items];
    const currentDragIndex = reordered.indexOf(dragged);
    if (currentDragIndex >= 0) {
        reordered.splice(currentDragIndex, 1);
    }
    reordered.splice(clamp(boundedTarget, 0, reordered.length), 0, dragged);

    reordered.forEach((item, idx) => {
        const badge = item.querySelector('.mod-number');
        if (badge) {
            badge.textContent = String(idx + 1);
        }
    });

    // When Sortable uses fallback mode, the dragged element can be an external clone.
    // Keep its badge synced with the predicted drop index too.
    if (draggedEl) {
        const draggedBadge = draggedEl.querySelector('.mod-number');
        if (draggedBadge) {
            draggedBadge.textContent = String(boundedTarget + 1);
        }
    }

    // Sortable fallback mode renders a floating clone outside the list.
    // Ensure that visible floating element's badge tracks the same target index.
    const floatingFallbacks = document.querySelectorAll('.sortable-fallback .mod-number');
    floatingFallbacks.forEach((badge) => {
        badge.textContent = String(boundedTarget + 1);
    });
}
