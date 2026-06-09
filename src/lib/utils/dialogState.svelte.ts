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
  onConfirm?: () => void;
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

  confirm(message: string, onConfirm: () => void, title = "Confirm Action") {
    this.state = { show: true, type: "confirm", title, message, onConfirm };
  }

  close() {
    this.state.show = false;
  }

  handleConfirm() {
    const cb = this.state.onConfirm;
    this.close();
    if (cb) cb();
  }
}

export const dialogStore = new DialogStore();
