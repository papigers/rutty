enum Status {
  CONNECTING,
  CONNECTED,
  DISCONNECTED,
}

const STATUS_CLASSES: Record<Status, string> = {
  [Status.CONNECTING]: "connecting",
  [Status.CONNECTED]: "connected",
  [Status.DISCONNECTED]: "disconnected",
}

export class StatusBanner {
  private _el: HTMLElement;
  private _textInterval?: number;

  constructor() {
    this._el = document.getElementById("status");
  }

  setConnecting() {
    this._clearTextInterval();
    this._setText("Connecting...")
    this._setStatus(Status.CONNECTING);
  }

  setConnected() {
    this._clearTextInterval();
    this._setText("Connected")
    this._setStatus(Status.CONNECTED);
  }

  setDisconnected(reason?: string, reconnect?: number) {
    this._clearTextInterval();
    let text = "Disconnected";
    if (reason !== null && reason !== "") {
      text += ` - ${reason}`
    }

    this._setText(text)
    this._setStatus(Status.DISCONNECTED);

    if (reconnect != null && reconnect >= 0) {
      let counter = 0;
      const textUpdater = () => {
        const seconds = reconnect - counter;
        counter += 1;
        if (seconds == 0) {
          this._clearTextInterval();
          return;
        }

        this._setText(text + ` (reconnecting in ${seconds}s)`);
      };
      this._textInterval = setInterval(textUpdater, 1000);
      textUpdater();
    }
  }

  private _setText(text: string) {
    this._el.textContent = text;
  }

  private _setStatus(status: Status) {
    this._el.className = STATUS_CLASSES[status];
  }

  private _clearTextInterval() {
    if (this._textInterval != null) {
      clearInterval(this._textInterval);
      this._textInterval = null;
    }
  }
}