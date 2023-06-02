import { Terminal, IDisposable, ITerminalAddon } from 'xterm';
import { StatusBanner } from './status';

const DELIMITER = ';';
const START_PREFIX = '0';
const INPUT_PREFIX = '1';
const RESIZE_PREFIX = '2';

interface Configuration {
  title?: string;
  reconnect?: number,
}

function createSocket() {
  const endpoint = `${window.location.origin.replace(/^http/, 'ws')}/ws`;
  const socket = new WebSocket(endpoint);
  socket.binaryType = 'arraybuffer';

  return socket;
}

export class WebSocketAddon implements ITerminalAddon {
  private _socket?: WebSocket
  private _terminal: Terminal;
  private _reconnect: number = -1;
  private _status: StatusBanner;
  private _disposables: IDisposable[] = [];

  constructor() {
    this._status = new StatusBanner();
  }

  public activate(terminal: Terminal): void {
    terminal.clear();
    terminal.focus();

    this._status.setConnecting();
    this._socket = createSocket();
    this._terminal = terminal;

    this._configure();

    this._disposables = [];
    this._disposables.push(addSocketListener(this._socket, 'open', () => this._onOpen()));
    this._disposables.push(addSocketListener(this._socket, 'message', (ev) => this._onMessage(ev)));
    this._disposables.push(addSocketListener(this._socket, 'close', (e) => this._dispose(e)));
    this._disposables.push(terminal.onData(data => this._sendData(data)));
    this._disposables.push(terminal.onBinary(data => this._sendBinary(data)));
    this._disposables.push(terminal.onResize(() => this._sendResize()));
    this._disposables.push(addWindowListener('resize', () => this._sendResize()));

    this._sendResize();
  }

  private _configure(): void {
    fetch("/config").then(res => res.json()).then(json => {
      const config = json as Configuration;
      if (config.reconnect !== null) {
        this._reconnect = config.reconnect;
      }
      if (config.title !== null) {
        document.title = config.title;
      }
    }).catch(err => {
      console.warn("Failed to read configuration: ", err);
    })
  }

  public dispose(): void {
    this._dispose(null);
  }

  private _dispose(closeEvent?: CloseEvent): void {
    this._status.setDisconnected(closeEvent?.reason, this._reconnect);
    this._terminal.blur();
    for (const d of this._disposables) {
      d.dispose();
    }

    if (this._reconnect >= 0) {
      const timeout = setTimeout(() => {
        this._reconnect = -1;
        this.activate(this._terminal);
      }, this._reconnect * 1000);

      this._disposables.push({ dispose: () => clearTimeout(timeout) });
    }
  }

  private _onOpen(): void {
    if (!this._checkOpenSocket()) {
      setTimeout(() => this._onOpen(), 1000);
      return;
    }

    this._status.setConnected();
    this._socket.send(`${START_PREFIX}${DELIMITER}${Math.round(this._terminal.rows)}${DELIMITER}${Math.round(this._terminal.cols)}`);
  }

  private _onMessage(ev: MessageEvent): void {
    const data: ArrayBuffer | string = ev.data;

    this._terminal.write(typeof data === 'string' ? data : new Uint8Array(data));
  }

  private _sendData(data: string): void {
    if (!this._checkOpenSocket()) {
      return;
    }

    this._socket.send(`${INPUT_PREFIX}${DELIMITER}${data}`);
  }

  private _sendResize(): void {
    if (!this._checkOpenSocket()) {
      return;
    }

    this._socket.send(`${RESIZE_PREFIX}${DELIMITER}${Math.round(this._terminal.rows)}${DELIMITER}${Math.round(this._terminal.cols)}`);
  }

  private _sendBinary(data: string): void {
    if (!this._checkOpenSocket()) {
      return;
    }
    const buffer = new Uint8Array(data.length + 2);
    buffer[0] = INPUT_PREFIX.charCodeAt(0);
    buffer[1] = DELIMITER.charCodeAt(0);
    for (let i = 0; i < data.length; ++i) {
      buffer[i + 2] = data.charCodeAt(i) & 255;
    }
    this._socket.send(buffer);
  }

  private _checkOpenSocket(): boolean {
    if (this._socket.readyState === WebSocket.OPEN) {
      return true;
    }

    console.warn(`Socket state is: ${this._socket.readyState}`);
    return false; ``
  }
}

function addSocketListener<K extends keyof WebSocketEventMap>(socket: WebSocket, type: K, handler: (this: WebSocket, ev: WebSocketEventMap[K]) => any): IDisposable {
  socket.addEventListener(type, handler);
  let disposed = false;
  return {
    dispose: () => {
      if (!handler || disposed) {
        // Already disposed
        return;
      }

      disposed = true;
      socket.removeEventListener(type, handler);
    }
  };
}

function addWindowListener<K extends keyof WindowEventMap>(type: K, handler: (this: Window, ev: WindowEventMap[K]) => any, options?: boolean | AddEventListenerOptions): IDisposable {
  window.addEventListener(type, handler, options);
  let disposed = false;
  return {
    dispose: () => {
      if (!handler || disposed) {
        // Already disposed
        return;
      }

      disposed = true;
      window.removeEventListener(type, handler);
    }
  };
}