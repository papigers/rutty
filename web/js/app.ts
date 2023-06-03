import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { WebLinksAddon } from 'xterm-addon-web-links';
import { CanvasAddon } from "xterm-addon-canvas";

import { WebSocketAddon } from "./websocket-addon";

const term = new Terminal({
  fontFamily: '"DejaVu Sans Mono", "Everson Mono", FreeMono, Menlo, Terminal, monospace, "Apple Symbols"',
});
const fitAddon = new FitAddon();

term.loadAddon(fitAddon);
term.loadAddon(new WebLinksAddon());
term.loadAddon(new CanvasAddon());
term.loadAddon(new WebSocketAddon());

term.open(document.getElementById("term"));

fitAddon.fit();

window.addEventListener('resize', () => {
  console.log('resize');
  fitAddon.fit();
});