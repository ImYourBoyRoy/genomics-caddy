import { isTauri } from '@tauri-apps/api/core';
import { LogicalPosition } from '@tauri-apps/api/dpi';
import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';

export interface DesktopContextMenuActions {
  hasReport: () => boolean;
  onReloadReport: () => void | Promise<void>;
  onOpenDiscovery: () => void;
  onOpenAiConsultation: () => void;
  onImportGenome: () => void | Promise<void>;
}

type NativeMenuItem = MenuItem | PredefinedMenuItem;

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : null;
  return !!element?.closest('input, textarea, [contenteditable="true"]');
}

async function createSeparator(): Promise<PredefinedMenuItem> {
  return PredefinedMenuItem.new({ item: 'Separator' });
}

async function createActionItem(
  id: string,
  text: string,
  action: () => void | Promise<void>,
): Promise<MenuItem> {
  return MenuItem.new({
    id,
    text,
    action: () => {
      void action();
    },
  });
}

async function openDesktopContextMenu(
  event: MouseEvent,
  actions: DesktopContextMenuActions,
): Promise<void> {
  const items: NativeMenuItem[] = [];

  if (isEditableTarget(event.target)) {
    items.push(
      await PredefinedMenuItem.new({ item: 'Cut', text: 'Cut' }),
      await PredefinedMenuItem.new({ item: 'Copy', text: 'Copy' }),
      await PredefinedMenuItem.new({ item: 'Paste', text: 'Paste' }),
      await PredefinedMenuItem.new({ item: 'SelectAll', text: 'Select all' }),
    );
  } else {
    items.push(await PredefinedMenuItem.new({ item: 'Copy', text: 'Copy selected text' }));
  }

  items.push(await createSeparator());

  if (actions.hasReport()) {
    items.push(
      await createActionItem('refresh-report', 'Refresh current report', actions.onReloadReport),
      await createActionItem('open-discovery', 'Explore research findings', actions.onOpenDiscovery),
      await createActionItem('open-ai-consultation', 'Open AI consultation', actions.onOpenAiConsultation),
    );
  } else {
    items.push(await createActionItem('import-genome', 'Import DNA export', actions.onImportGenome));
  }

  const menu = await Menu.new({ items });
  try {
    await menu.popup(new LogicalPosition(event.clientX, event.clientY));
  } finally {
    await menu.close();
  }
}

/**
 * Installs the desktop-only application context menu. Web preview keeps the
 * browser's normal context menu so development tools remain available there.
 */
export function installDesktopContextMenu(actions: DesktopContextMenuActions): () => void {
  if (!isTauri()) return () => {};

  const handler = (event: MouseEvent) => {
    event.preventDefault();
    void openDesktopContextMenu(event, actions).catch((error: unknown) => {
      console.warn('Desktop context menu failed:', error);
    });
  };

  document.addEventListener('contextmenu', handler);
  document.documentElement.dataset.contextMenu = 'genomics-caddy';

  return () => {
    document.removeEventListener('contextmenu', handler);
    delete document.documentElement.dataset.contextMenu;
  };
}
