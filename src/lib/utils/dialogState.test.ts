import { afterEach, describe, expect, it, vi } from 'vitest';

vi.stubGlobal('$state', <T>(value: T): T => value);
const { dialogStore } = await import('./dialogState.svelte');

afterEach(() => {
  dialogStore.close();
});

describe('dialogStore async confirmation', () => {
  it('keeps the confirmation visible and busy until its action completes', async () => {
    let finishAction!: () => void;
    const action = new Promise<void>((resolve) => {
      finishAction = resolve;
    });
    dialogStore.confirm('Delete this profile?', () => action, 'Delete profile');

    const pending = dialogStore.handleConfirm();

    expect(dialogStore.state.show).toBe(true);
    expect(dialogStore.state.busy).toBe(true);
    dialogStore.close();
    expect(dialogStore.state.show).toBe(true);
    finishAction();
    await pending;

    expect(dialogStore.state.show).toBe(false);
  });

  it('preserves an error dialog opened by a failed async confirmation', async () => {
    dialogStore.confirm('Delete this profile?', async () => {
      dialogStore.alert('The profile could not be deleted.', 'Delete failed');
    });

    await dialogStore.handleConfirm();

    expect(dialogStore.state).toMatchObject({
      show: true,
      type: 'alert',
      title: 'Delete failed',
      message: 'The profile could not be deleted.',
    });
  });

  it('does not start a second confirmation while an async action is pending', async () => {
    let finishAction!: () => void;
    const action = new Promise<void>((resolve) => {
      finishAction = resolve;
    });
    const callback = vi.fn(() => action);
    dialogStore.confirm('Delete this profile?', callback);

    const pending = dialogStore.handleConfirm();
    await dialogStore.handleConfirm();
    expect(callback).toHaveBeenCalledOnce();
    finishAction();
    await pending;
  });

  it('treats choice Cancel as abort instead of picking an action', async () => {
    const onChoice = vi.fn();
    dialogStore.choice(
      'Use this folder?',
      [
        { id: 'move', label: 'Move library' },
        { id: 'use', label: 'Use empty folder' },
      ],
      onChoice,
      'Change library folder',
    );

    dialogStore.close({ cancelled: true });
    expect(onChoice).not.toHaveBeenCalled();
    expect(dialogStore.state.show).toBe(false);
  });

  it('runs the selected choice and ignores a second click while busy', async () => {
    let finishAction!: () => void;
    const action = new Promise<void>((resolve) => {
      finishAction = resolve;
    });
    const onChoice = vi.fn(() => action);
    dialogStore.choice(
      'Use this folder?',
      [
        { id: 'move', label: 'Move library' },
        { id: 'use', label: 'Use empty folder' },
      ],
      onChoice,
    );

    const pending = dialogStore.handleChoice('move');
    await dialogStore.handleChoice('use');
    expect(onChoice).toHaveBeenCalledOnce();
    expect(onChoice).toHaveBeenCalledWith('move');
    finishAction();
    await pending;
    expect(dialogStore.state.show).toBe(false);
  });
});
