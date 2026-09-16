// ./src/lib/utils/dialogState.svelte.ts
/**
 * Dialog State Store.
 * Purpose: Manage global alert and confirm dialog states reactively for Svelte 5.
 * Key Inputs: Trigger methods `alert` and `confirm`.
 * Key Outputs: Reactive `state` object.
 * Operational Notes: Replaces standard browser window alert/confirm with custom UI dialogs.
 */

export interface DialogState {
  show: boolean;
  type: "alert" | "confirm";
  title: string;
  message: string;
  busy?: boolean;
  onConfirm?: () => void | Promise<void>;
  onCancel?: () => void;
}

class DialogStore {
  // Svelte 5 reactive state rune
  state = $state<DialogState>({
    show: false,
    type: "alert",
    title: "",
    message: "",
  });

  alert(message: string, title = "Genomics Caddy AI") {
    this.state = { show: true, type: "alert", title, message };
  }

  confirm(
    message: string,
    onConfirm: () => void | Promise<void>,
    title = "Confirm Action",
    onCancel?: () => void,
  ) {
    this.state = { show: true, type: "confirm", title, message, onConfirm, onCancel };
  }

  close(options: { cancelled?: boolean } = {}) {
    if (this.state.busy) return;
    const cancel = this.state.onCancel;
    this.state = {
      show: false,
      type: this.state.type,
      title: "",
      message: "",
    };
    if (options.cancelled && cancel) cancel();
  }

  async handleConfirm(): Promise<void> {
    const cb = this.state.onConfirm;
    if (!cb || this.state.busy) return;

    this.state = { ...this.state, busy: true };
    try {
      await cb();
    } catch (error) {
      if (this.state.onConfirm === cb) {
        this.alert(`Action failed: ${String(error)}`, 'Genomics Caddy');
      }
    } finally {
      // An async action may replace the confirmation with its own error dialog.
      // Only close the original dialog when it is still the active one.
      if (this.state.onConfirm === cb) {
        this.state = { ...this.state, busy: false };
        this.close();
      }
    }
  }
}

export const dialogStore = new DialogStore();
